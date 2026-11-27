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
# Exit 0: all documented. Exit 1: the list of items without `# Examples`.
set -euo pipefail

dir="${1:-studio/src}"

python3 - "$dir" <<'PY'
import os
import re
import sys

ITEM = re.compile(r'^\s*pub\s+(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?(fn|struct|enum|trait|const|static|type|union)\s+(\w+)')
PUB_MOD = re.compile(r'^\s*pub\s+mod\s+(\w+)\s*;')


def production_lines(path):
    with open(path, encoding="utf-8") as source:
        lines = source.read().split("\n")
    for i, line in enumerate(lines):
        if line.strip() == "#[cfg(test)]" and i + 1 < len(lines) and re.match(r'\s*mod tests\b', lines[i + 1]):
            return lines[:i]
    return lines


def has_examples(lines, start):
    i = start - 1
    while i >= 0 and lines[i].strip().startswith("#["):
        i -= 1
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
        if path.endswith("lib.rs"):
            continue
        item = ITEM.match(line)
        if item and not has_examples(lines, number):
            failures.append(f"{path}:{number + 1}: pub {item.group(1)} {item.group(2)} has no # Examples")

if failures:
    print("\n".join(failures))
    sys.exit(1)
PY
