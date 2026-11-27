#!/usr/bin/env bash
# Shared scanner code of the doc gates `errors-doc.sh` and `examples-doc.sh`.
#
# Usage: source this file, then call `doc_items_python`. The function prints
# the Python helper functions. A gate puts its own Python code after them and
# gives the result to `python3 -`. Example:
#
#   { doc_items_python; cat <<'PY'
#   ...the rule of the gate...
#   PY
#   } | python3 - "$dir"
#
# The helpers are:
# - production_lines(path): the lines of a file without its test module (from
#   `#[cfg(test)]` followed by `mod tests`, to the end).
# - has_doc_section(lines, start, heading, skip_comments): True if the doc
#   comment above line `start` has a line that starts with `/// <heading>`.
#   Attributes may sit between the doc comment and the item. An attribute can
#   have several lines, for example `#[expect(` ... `)]`. If `skip_comments`
#   is True, plain `//` comments may also sit between.
#
# Keep the rules of both gates in their own files. This file has only the code
# that both gates need, so a fix of the scanner applies to both gates.

doc_items_python() {
    cat <<'PY'
import re


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


def skip_attributes(lines, i, skip_comments):
    # Go up from line `i` over the attributes (and the `//` comments, if `skip_comments`). Return the first other line.
    while i >= 0:
        text = lines[i].strip()
        if text.startswith("#[") or (skip_comments and text.startswith("//") and not text.startswith("///")):
            i -= 1
        elif text.endswith("]"):
            start = attribute_start(lines, i)
            if start is None:
                return i
            i = start - 1
        else:
            return i
    return i


def has_doc_section(lines, start, heading, skip_comments):
    i = skip_attributes(lines, start - 1, skip_comments)
    while i >= 0 and lines[i].strip().startswith("///"):
        if lines[i].strip().startswith("/// " + heading):
            return True
        i -= 1
    return False
PY
}
