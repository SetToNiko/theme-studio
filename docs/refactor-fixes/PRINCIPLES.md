# Refactor Principles: Theme Studio, the new kit rules

<!-- The refactor-kit setup wrote this file from kit:catalog/principles.md. You can edit it. Run /refactor-kit setup to do the interview again. -->

These principles set HOW the refactor writes the code. `GOAL.md` sets WHAT the refactor changes.
- A principle never justifies work outside the goal scope.
- The goal never justifies a loss of correctness, or code that fails the audience test.

## 1. Audience: A1 Beginner (junior developer)

**The reader:**
- knows the basics of the language: variables, functions, types (structs and enums), loops, collections, and simple traits
- can read a stack trace
- possibly does *not* know these topics:
  - design patterns by name, async internals, and long iterator chains
  - generics with trait bounds, associated types, and lifetimes in signatures
  - macros, `build.rs`, procedural macros, and the behavior of attributes and derive macros

**The audience test:** without help from another person, can the reader do these three things?
1. Open the file.
2. Understand what the file does and why, in a few minutes.
3. Make a small change safely.

**Rules:**
- **Advanced techniques need an explanation.** The refactorer can use a pattern, a generic, or a language feature above beginner level when it gives a clear benefit. In that case, the refactorer must:
  1. Hide the technique behind a simple interface with a plain name.
  2. Add a short comment at that location. The comment gives the name of the technique, the reason for it, and how to work with it. Example: `// Trait object: each app integration is a type that implements AppIntegration, so you can add a new app without a change to this file. To add one: ...`
  3. List the technique in the glossary "Concepts used in this codebase" in ARCHITECTURE.md.
- **Few layers of indirection.** To understand one behavior, a reader follows a maximum of 3 jumps of the type trait → implementation → delegate.
- **Explicit code, not clever code.** A plain `for` loop is better than a dense chain of 5 iterator adapters. A named variable is better than a nested expression. A simple `if` is better than a nested expression in a closure.
- **Clear structure.** Folder names and file names tell the reader where to look. Use names such as `fonts/` and `keymap/`, not `core/impl/internal/`.
- **Comments give the intent.** Each public type has a one-line comment that gives its purpose. Logic that is not obvious has a comment that gives the reason.
- **Each implicit behavior has a pointer.** If behavior occurs implicitly (attributes, derive macros, macros, `build.rs`, generated code), add a comment that gives the location of the wiring.
- **Documentation.** The docs use plain language. ARCHITECTURE.md has a "How to read this codebase" guide and a glossary of each advanced concept.

## 2. Targets

### T1: SOLID

| Principle | Target | Sign of a violation |
|---|---|---|
| **S**ingle Responsibility | Each type or module has one reason to change | "Manager" or "Util" types, files that mix UI, logic, and I/O, functions with "and" in their description |
| **O**pen/Closed | You add new behavior with new code, not with changes to existing code | `match` or `if-else` chains on a type or kind that grow with each feature |
| **L**iskov Substitution | Each type that implements a trait works at each location where the code expects the trait | Trait methods that panic with "not supported", checks for one specific type that implements the trait |
| **I**nterface Segregation | Traits are small and focused. Callers depend only on the methods that they use | Traits with many methods, where implementations leave some methods empty |
| **D**ependency Inversion | High-level logic depends on abstractions. Callers pass in the concrete values or trait objects | Concrete types made inside business logic, global state, hardcoded file, process, or network calls in logic |

### T2: Data-driven design

Move *variation* out of the code and into *data*.
- **Data controls behavior, not branches.** Replace long conditionals on kinds or types with a lookup table, a registry, or a configuration.
- **Keep data separate from logic.** Put constants, rules, content, and tuning values in data structures or configuration files.
- **Model the domain explicitly.** Use typed structs and enums. Do not use loose maps, tuples, or parallel arrays.
- **Single source of truth.** Define each piece of data one time.
- **Keep the completeness check.** A `match` on an enum fails to compile when a variant is missing. A table does not. When a table replaces such a `match`, keep one exhaustive check. Example: a `match` with no default arm that lists each variant. A test that gets the variants from another source is also good. A test that loops over the table cannot find a missing row.
- **Use evidence to set priorities.** Churn, file size, duplication, bug history, and test coverage show where to start.

### T3: Readability

- Names tell *what* and *why*, not *how*. Example: `calculate_damage`, not `do_calc2`.
- Each function does one thing. Guideline: 30 lines of code. Longer only with a verified reason (section 3).
- **Group the code by purpose.** Put the lines that do one task together, and put one empty line between two groups. In a file, put related items next to each other, for example a type and then its functions. A short comment above a group can give its purpose. Empty lines and comments do not count toward a line limit (section 3), so a limit is never a reason to remove them.
- Shallow nesting: use early returns and guard clauses. Limit: 4 levels.
- No magic numbers or strings: use named constants or data.
- Comments tell *why*. The code tells *what*. Remove commented-out code.
- Formatting and patterns are consistent across the codebase.

### T4: Maintainability

- **DRY, but not too early.** Extract duplicate code at the third occurrence, or when the copies must change together.
- **Small units.** Limit: files of 500 lines of code or fewer, without the `#[cfg(test)]` module.
- **Low coupling, high cohesion.** Related code is in the same location. Modules communicate through narrow interfaces.
- **Testability.** You can test logic without a UI, network, or database.
- **No dead code.** Remove unused functions, flags, and parameters.
- **Explicit errors.** Handle or propagate each failure on purpose. Never ignore a failure silently.

### T5: Modularity

- Each module has a clear boundary and a defined public API. Its internals are private.
- Dependencies go in one direction, for example UI → domain → data. There are no cycles.
- You can understand, test, and replace a module independently.
- The folder structure follows features or layers consistently.

### T6: Extensibility

- **Extension points** exist where variation is expected: traits, registries, plugin hooks, and configuration.
- To add a new variant, you **add a file or a data entry**. You do not edit many existing files.
- Use composition, not inheritance, where possible.
- **YAGNI:** add extension points only where the plan or the history shows real variation.

### T7: Error handling

- Each error message says what failed, with which input, and what the user can do.
- Errors keep their cause. Do not replace a detailed error with a general error.
- One location per layer converts low-level errors into domain errors.
- Expected failures (bad input, missing file) use `Result`, not a panic.

## 3. Limits

| Limit | Value | Kind | Later target |
|---|---|---|---|
| Jumps of indirection to understand one behavior | 3 | hard | none |
| Lines of code in one function | 30 | guideline: longer only with a verified reason | none |
| Levels of nesting | 4 | hard | none |
| Lines of code in one file, without the `#[cfg(test)]` module | 500 | hard (gate `file-limits`) | none |
| Modules in one module folder, not counting `mod.rs` | 10 | hard (review) | none |

Each line limit counts lines of code only. Empty lines and comment lines (doc comments too) do not count. A line with code and a comment counts as one line. Use empty lines and comments to group the code by purpose: a limit is never a reason to remove them.

### Verified exceptions

This project names the marker of the function-length exception "Long function".

1. **The refactorer writes a reason.** A function of more than 30 lines of code gets a reason comment directly above it: `// Long function: <reason>.` A function of more than 100 lines also triggers `clippy::too_many_lines`. Put the reason in `#[expect(clippy::too_many_lines, reason = "...")]`.
2. **The reviewer checks the reason.** It reports one of these results:
   - **verified:** the reason is true and specific. Dividing the code would make it harder for the audience to read.
   - **finding:** the reason is missing, vague, or false. The finding gives a proposal to divide the code.
3. **The review action adds the marker.** For each verified exception, it changes the comment to: `// Long function (reason verified, phase <N>): <reason>.` The reviewer never edits files. Thus, the review action writes the marker.
4. **Later reviews accept a verified exception.** They check it again only in these cases:
   - **An architecture rewrite:** a plan step or a phase whose stated purpose is to restructure the module that contains the exception.
   - **The code grows:** the function is longer or deeper than at the verification.
   - **The reason is no longer true.**

The same process applies to each lint that a project rule permits with a reason (`#[expect(<lint>, reason = "...")]`), and to each deviation from the Rust API Guidelines. A deviation gives the item ID in its reason. Example: `reason = "C-GETTER: ..."`.

## 4. Priority

If targets conflict, use this order:

1. **Correctness:** the behavior stays the same. Tests prove it.
2. **Audience test:** the code passes the A1 audience test.
3. **Readability:** an experienced reader understands the code quickly.
4. **Maintainability, modularity, and error handling:** changes stay local, and failures are explicit.
5. **Extensibility:** only where variation is real.

A simpler design that is slightly less "pure" is better than a textbook pattern that the audience cannot follow.

## 5. Project rules

1. All system paths go through `studio/src/model/paths.rs` (Rust) or `scripts/lib/paths.sh` (bash). New code never hard-codes a path. The two files use the same rules and give the same paths.
2. Only one location runs app reload commands (`pkill`, `i3-msg`, and others).
3. Changed shell scripts pass `shellcheck`. Changed Rust code passes `cargo clippy --all-targets -- -D warnings` with the groups pedantic, style, and correctness (`nix flake check` fails on any warning).
4. Rust documentation follows the rustdoc style strictly. Refer to the project-conventions skill, section "Documentation style".
5. No `pub use` re-exports. All `impl App` blocks stay in `app/`.
6. The switch scripts (`scripts/*.sh`, `scripts/lib/*.sh`) do not use `set -euo pipefail`, because it changes their behavior.
7. Each `mod.rs` holds only a `//!` header and module declarations, with a `///` comment on each declaration. A gate checks it (GOAL.md, criterion 2).
8. Each module and item has the smallest visibility that works. `pub` is only on the API of the library target (GOAL.md, criterion 3).
