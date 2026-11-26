#!/usr/bin/env bash
# Gate: each production function that returns `Result` needs a `# Errors`
# section in the doc comment directly above it (attributes may sit between).
#
# Usage: bash scripts/dev/errors-doc.sh [DIR]   (default: studio/src)
#
# Skipped: files named tests.rs, files in a tests/ folder, and the test module
# of a file (from `#[cfg(test)]` followed by `mod tests`, to the end).
# Exit 0: all documented. Exit 1: the list of functions without `# Errors`.
set -euo pipefail

dir="${1:-studio/src}"

python3 - "$dir" <<'PY'
import os
import re
import sys

FN = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+(\w+)')


def production_lines(path):
    lines = open(path, encoding="utf-8").read().split("\n")
    for i, line in enumerate(lines):
        if line.strip() == "#[cfg(test)]" and i + 1 < len(lines) and re.match(r'\s*mod tests\b', lines[i + 1]):
            return lines[:i]
    return lines


def signature(lines, start):
    # From `fn` to the `{` or `;` that follows the closing `)` of the parameters.
    text = ""
    for line in lines[start:]:
        text += line + "\n"
        if text.count("(") == text.count(")") and ("{" in line or line.rstrip().endswith(";")):
            break
    return text


def returns_result(sig):
    # The return type follows the `)` that closes the parameter list.
    k = sig.find("(", sig.index("fn "))
    depth = 0
    while k < len(sig):
        depth += (sig[k] == "(") - (sig[k] == ")")
        if depth == 0:
            break
        k += 1
    after = re.match(r'\s*->\s*(.*)', sig[k + 1:], re.S)
    return after is not None and re.match(r'(?:\w+::)*Result\b', after.group(1)) is not None


def has_errors_doc(lines, start):
    i = start - 1
    while i >= 0 and (lines[i].strip().startswith("#[") or lines[i].strip().startswith("//") and not lines[i].strip().startswith("///")):
        i -= 1
    while i >= 0 and lines[i].strip().startswith("///"):
        if lines[i].strip().startswith("/// # Errors"):
            return True
        i -= 1
    return False


bad = []
for root, _, files in os.walk(sys.argv[1]):
    if "tests" in root.split(os.sep):
        continue
    for name in sorted(files):
        if not name.endswith(".rs") or name == "tests.rs":
            continue
        path = os.path.join(root, name)
        lines = production_lines(path)
        for i, line in enumerate(lines):
            m = FN.match(line)
            if m and returns_result(signature(lines, i)) and not has_errors_doc(lines, i):
                bad.append(f"{path}:{i + 1}: fn {m.group(1)} returns Result but has no `# Errors` doc")

for b in sorted(bad):
    print(b)
sys.exit(1 if bad else 0)
PY
