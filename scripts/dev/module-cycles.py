#!/usr/bin/env python3
"""Count the dependency cycles between the studio's Rust modules.

Usage: python3 scripts/dev/module-cycles.py [studio/src]

Each .rs file under the folder (also in subfolders) is a module, named by its
file name (mod.rs: its folder's name). A module depends on another module when
its code names `crate::...::<other>`. The test module (from `#[cfg(test)]` followed by `mod tests`) and files named
tests.rs do not count.
A cycle group is a set of modules that all depend on each other, directly
or through other modules in the group. The refactor wants zero groups.

Why not `cargo modules dependencies --acyclic`: it sees only `use`
statements, not paths such as `crate::hit::HitMap` in the code, and it
stops at the first cycle. This script counts all of them.

Prints the number of groups, the number of modules in them, and each group.
"""

import os
import re
import sys


def module_name(path, src_dir):
    """The module name of a .rs file: its file name, or the folder name for mod.rs.

    Example: app/mod.rs -> app, app/layout.rs -> layout, core/color.rs -> color.
    Folders do not count, so the names stay the same when files move into
    folders (and `crate::color` through a re-export still finds `color`).
    """
    relative = os.path.relpath(path, src_dir)
    folder, file_name = os.path.split(relative)
    if file_name == "mod.rs":
        return os.path.basename(folder)
    return file_name[: -len(".rs")]


def without_test_module(text):
    """The code of a file without its test module.

    The test module starts at a line `#[cfg(test)]` that is followed by a line
    `mod tests`. Only that cut counts: a `#[cfg(test)]` on a single function or
    field earlier in the file must not hide the code after it.
    """
    match = re.search(r"^#\[cfg\(test\)\]\s*\n\s*mod tests\b", text, re.MULTILINE)
    if match is None:
        return text
    return text[: match.start()]


def read_dependencies(src_dir):
    """Return {module: set of modules that it names}, for each .rs file.

    Files named tests.rs hold only tests and do not count.
    A path such as `crate::app::layout::Layout` counts for the deepest module
    in it (here: layout).
    """
    production_code = {}
    for folder, _subfolders, file_names in os.walk(src_dir):
        for file_name in sorted(file_names):
            if not file_name.endswith(".rs") or file_name == "tests.rs":
                continue
            path = os.path.join(folder, file_name)
            with open(path, encoding="utf-8") as f:
                text = f.read()
            production_code[module_name(path, src_dir)] = without_test_module(text)

    dependencies = {}
    for module, text in production_code.items():
        named = set()
        for path in re.findall(r"crate((?:::\w+)+)", text):
            segments = path.strip(":").split("::")
            known = [segment for segment in segments if segment in production_code]
            if known:
                named.add(known[-1])
        named.discard(module)
        dependencies[module] = named
    return dependencies


def cycle_groups(dependencies):
    """Return the groups of modules that depend on each other.

    This is Tarjan's algorithm for strongly connected components: one walk
    through the graph that gives each module a number in visit order, and
    the lowest number that it can reach. Modules that reach back to the
    same lowest number form one group.
    """
    order = {}  # module -> its number in the visit order
    lowest = {}  # module -> lowest number that it can reach
    stack = []
    on_stack = set()
    groups = []

    def visit(module):
        order[module] = len(order)
        lowest[module] = order[module]
        stack.append(module)
        on_stack.add(module)
        for other in dependencies.get(module, ()):
            if other not in order:
                visit(other)
                lowest[module] = min(lowest[module], lowest[other])
            elif other in on_stack:
                lowest[module] = min(lowest[module], order[other])
        if lowest[module] == order[module]:
            group = []
            while True:
                member = stack.pop()
                on_stack.discard(member)
                group.append(member)
                if member == module:
                    break
            if len(group) > 1:
                groups.append(sorted(group))

    for module in sorted(dependencies):
        if module not in order:
            visit(module)
    return groups


def main():
    src_dir = sys.argv[1] if len(sys.argv) > 1 else "studio/src"
    groups = cycle_groups(read_dependencies(src_dir))
    modules_in_cycles = sum(len(group) for group in groups)
    print(f"cycle groups: {len(groups)}, modules in cycles: {modules_in_cycles}")
    for group in groups:
        print("  " + " ".join(group))


if __name__ == "__main__":
    main()
