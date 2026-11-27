#!/usr/bin/env bash
# Gate: each `pub` item that lib.rs exports has a `# Examples` section in the
# doc comment directly above it (attributes may sit between).
#
# Usage: bash scripts/dev/examples-doc.sh [DIR]   (default: studio/src)
#
# The script starts at each `pub mod` line of DIR/lib.rs. It reads the file of
# that module (name.rs or name/mod.rs), checks each `pub` item in it, and then
# follows the `pub mod` lines of that file. Items with a limited visibility,
# such as `pub(crate)`, are not exported and are skipped. The test module of a
# file (from `#[cfg(test)]` followed by `mod tests`, to the end) is skipped.
# An attribute between the doc comment and the item can have several lines,
# for example `#[expect(` ... `)]`.
#
# Known limits:
# - A `pub fn` in an `impl` block counts as an item, also when its type is
#   private. Make such a method `pub(crate)`, or give it `# Examples`.
# - The script does not read an inline module (`pub mod name { ... }`). It
#   fails on one: move the module into its own file.
#
# Exit 0: all documented. Exit 1: the list of items without `# Examples`.
#
# --selftest checks a sample tree with each difficult case and compares the
# result with the known list of failures.
set -euo pipefail

if [ "${1:-}" = "--selftest" ]; then
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    cat > "$tmp/lib.rs" <<'SAMPLE'
//! The sample crate.
/// The public module.
pub mod a;
/// A private module: the script does not read it.
mod hidden;
SAMPLE
    cat > "$tmp/a.rs" <<'SAMPLE'
//! The public module.

/// Has examples, with a one-line attribute.
///
/// # Examples
#[must_use]
pub fn one_line() {}

/// Has examples, with a multi-line attribute.
///
/// # Examples
#[expect(
    clippy::cast_possible_truncation,
    reason = "the sample"
)]
pub fn multi_line() {}

/// No examples.
pub unsafe extern "C" fn no_examples_extern() {}

/// No examples.
pub struct NoExamples;

/// Not exported.
pub(crate) fn not_exported() {}

/// An inline module.
pub mod inline {}

#[cfg(test)]
mod tests {
    pub fn in_tests() {}
}
SAMPLE
    printf 'pub fn hidden() {}\n' > "$tmp/hidden.rs"
    expected='inline module inline: move it into its own file. The script does not read it.
pub fn no_examples_extern has no # Examples
pub struct NoExamples has no # Examples'
    got=$(bash "$0" "$tmp" | sed 's/^[^ ]*: //' | sort || true)
    if [ "$got" != "$expected" ]; then
        printf 'examples-doc self-test failed. Expected:\n%s\nGot:\n%s\nFix the Python program.\n' "$expected" "$got" >&2
        exit 1
    fi
    exit 0
fi

dir="${1:-studio/src}"

python3 - "$dir" <<'PY'
import os
import re
import sys

ITEM = re.compile(r'^\s*pub\s+(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+"[^"]*"\s+)?(fn|struct|enum|trait|const|static|type|union)\s+(\w+)')
PUB_MOD = re.compile(r'^\s*pub\s+mod\s+(\w+)\s*;')
INLINE_MOD = re.compile(r'^\s*pub\s+mod\s+(\w+)\s*\{')


def production_lines(path):
    with open(path, encoding="utf-8") as source:
        lines = source.read().split("\n")
    for i, line in enumerate(lines):
        if line.strip() == "#[cfg(test)]" and i + 1 < len(lines) and re.match(r'\s*mod tests\b', lines[i + 1]):
            return lines[:i]
    return lines


def attribute_start(lines, end):
    # The `#[` line of a multi-line attribute that ends on line `end`, or None.
    j = end - 1
    while j >= 0:
        text = lines[j].strip()
        if text.startswith("#["):
            return j
        if text == "" or text.startswith("///") or text.endswith((";", "{", "}")):
            return None
        j -= 1
    return None


def skip_attributes(lines, i):
    # Go up from line `i` over the attributes. Return the first other line.
    while i >= 0:
        text = lines[i].strip()
        if text.startswith("#["):
            i -= 1
        elif text.endswith("]"):
            start = attribute_start(lines, i)
            if start is None:
                return i
            i = start - 1
        else:
            return i
    return i


def has_examples(lines, start):
    i = skip_attributes(lines, start - 1)
    while i >= 0 and lines[i].strip().startswith("///"):
        if lines[i].strip().startswith("/// # Examples"):
            return True
        i -= 1
    return False


def module_file(folder, name):
    for candidate in (os.path.join(folder, name + ".rs"), os.path.join(folder, name, "mod.rs")):
        if os.path.isfile(candidate):
            return candidate
    return None


root = sys.argv[1]
queue = [os.path.join(root, "lib.rs")]
failures = []
while queue:
    path = queue.pop()
    lines = production_lines(path)
    # The folder where the child modules of this file live.
    base = os.path.basename(path)
    folder = os.path.dirname(path) if base in ("lib.rs", "mod.rs") else os.path.join(os.path.dirname(path), base[:-3])
    for number, line in enumerate(lines):
        mod = PUB_MOD.match(line)
        if mod:
            child = module_file(folder, mod.group(1))
            if child is None:
                failures.append(f"{path}:{number + 1}: no file for module {mod.group(1)}")
            else:
                queue.append(child)
            continue
        inline = INLINE_MOD.match(line)
        if inline:
            failures.append(f"{path}:{number + 1}: inline module {inline.group(1)}: move it into its own file. The script does not read it.")
            continue
        if path.endswith("lib.rs"):
            continue
        item = ITEM.match(line)
        if item and not has_examples(lines, number):
            failures.append(f"{path}:{number + 1}: pub {item.group(1)} {item.group(2)} has no # Examples")

if failures:
    print("\n".join(failures))
    sys.exit(1)
PY
