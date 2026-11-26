#!/usr/bin/env python3
"""Count the dependency cycles between the studio's Rust modules.

Usage: python3 scripts/dev/module-cycles.py [studio/src]

A module depends on another module when its code names `crate::<other>`.
The test module (everything after `#[cfg(test)]`) does not count.
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


def read_dependencies(src_dir):
    """Return {module: set of modules that it names}, for each .rs file."""
    dependencies = {}
    for file_name in sorted(os.listdir(src_dir)):
        if not file_name.endswith(".rs"):
            continue
        module = file_name[: -len(".rs")]
        with open(os.path.join(src_dir, file_name), encoding="utf-8") as f:
            text = f.read()
        production_code = text.split("#[cfg(test)]")[0]
        named = set(re.findall(r"crate::(\w+)", production_code))
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
