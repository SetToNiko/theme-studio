#!/usr/bin/env python3
"""Count the dependency cycles between the studio's Rust modules.

Usage: python3 scripts/dev/module-cycles.py [studio/src]

Each .rs file under the folder (also in subfolders) is a module, named by its
full module path (app/state/nav.rs: app::state::nav; mod.rs: its folder's
path). A module depends on another module when its code names that path after
`crate::`. The test module (from `#[cfg(test)]` followed by `mod tests`) and files named
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


def module_path(path, src_dir):
    """The module path of a .rs file, as the code writes it after `crate::`.

    Example: main.rs -> "" (the crate root), color.rs -> color,
    app/mod.rs -> app, app/state/nav.rs -> app::state::nav.
    The full path keeps two files with the same name apart, for example
    app/pages/settings/mod.rs (app::pages::settings) and settings_page/mod.rs (settings_page).
    """
    relative = os.path.relpath(path, src_dir)[: -len(".rs")]
    parts = relative.split(os.sep)
    if parts[-1] in ("mod", "main"):
        parts = parts[:-1]
    return "::".join(parts)


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

    Files named tests.rs hold only tests and do not count; neither does the
    crate root (main.rs), which names every module.
    A path such as `crate::app::state::nav::Page` counts for the longest
    module path that it starts with (here: app::state::nav).
    """
    production_code = {}
    for folder, _subfolders, file_names in os.walk(src_dir):
        for file_name in sorted(file_names):
            if not file_name.endswith(".rs") or file_name == "tests.rs":
                continue
            path = os.path.join(folder, file_name)
            module = module_path(path, src_dir)
            if module == "":
                continue
            with open(path, encoding="utf-8") as f:
                production_code[module] = without_test_module(f.read())

    dependencies = {}
    for module, text in production_code.items():
        named = set()
        # `pub(in crate::app)` limits who can use an item; it is not a dependency.
        code = re.sub(r"pub\(in crate(?:::\w+)+\)", "pub", text)
        for path in re.findall(r"crate((?:::\w+)+)", code):
            segments = path.strip(":").split("::")
            for length in range(len(segments), 0, -1):
                candidate = "::".join(segments[:length])
                if candidate in production_code:
                    named.add(candidate)
                    break
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
