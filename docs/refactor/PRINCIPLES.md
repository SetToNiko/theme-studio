# Refactor Principles: Theme Studio

<!-- The refactor-kit setup wrote this file from the kit catalog. You can edit it. Run /refactor-kit setup to do the interview again. -->

These principles set HOW the refactor writes the code. `GOAL.md` sets WHAT the refactor changes.
- A principle never justifies work outside the goal scope.
- The goal never justifies a loss of correctness, or code that fails the audience test.

## 1. Audience: A1 Beginner (junior developer)

**The reader:**
- knows the basics of the language: variables, functions, classes, loops, collections, and simple interfaces
- can read a stack trace
- possibly does *not* know design patterns by name, advanced generics, reflection, annotation behavior, DI frameworks, async internals, long functional chains, or metaprogramming

**The audience test:** without help from another person, can the reader do these three things?
1. Open the file.
2. Understand what the file does and why, in a few minutes.
3. Make a small change safely.

**Rules:**
- **Advanced techniques need an explanation.** The refactorer can use a pattern, a generic, or a language feature above beginner level when it gives a clear benefit. In that case, the refactorer must:
  1. Hide the technique behind a simple interface with a plain name.
  2. Add a short comment at that location. The comment gives the name of the technique, the reason for it, and how to work with it. Example: `// Strategy pattern: each AttackType has its own class, so you can add new types without a change to this file. To add one: ...`
  3. List the technique in the glossary "Concepts used in this codebase" in ARCHITECTURE.md.
- **Few layers of indirection.** To understand one behavior, a reader follows a maximum of 3 jumps of the type interface → implementation → delegate.
- **Explicit code, not clever code.** A plain `for` loop is better than a dense chain of 5 functional operators. A named variable is better than a nested expression. A simple `if` is better than a ternary in a lambda.
- **Clear structure.** Folder names and file names tell the reader where to look. Use names such as `battle/` and `save/`, not `core/impl/internal/`.
- **Comments give the intent.** Each public class has a one-line comment that gives its purpose. Logic that is not obvious has a comment that gives the reason.
- **Each implicit framework behavior has a pointer.** If behavior occurs implicitly (annotations, DI wiring, reflection, generated code), add a comment that gives the location of the wiring.
- **Documentation.** The docs use plain language. ARCHITECTURE.md has a "How to read this codebase" guide and a glossary of each advanced concept.

## 2. Targets

### T1: SOLID

| Principle | Target | Sign of a violation |
|---|---|---|
| **S**ingle Responsibility | Each class or module has one reason to change | "Manager" or "Util" classes, files that mix UI, logic, and I/O, functions with "and" in their description |
| **O**pen/Closed | You add new behavior with new code, not with changes to existing code | `when`, `switch`, or `if-else` chains on a type or kind that grow with each feature |
| **L**iskov Substitution | A subtype works at each location where the code expects the parent type | Overrides that throw "not supported", `instanceof` checks on subtypes |
| **I**nterface Segregation | Interfaces are small and focused. Clients depend only on the methods that they use | Interfaces with many methods, where implementations leave some methods empty |
| **D**ependency Inversion | High-level logic depends on abstractions. Callers inject the concrete classes | `new ConcreteThing()` in business logic, static singletons, hardcoded database or network calls in logic |

### T2: Data-driven design

Move *variation* out of the code and into *data*.
- **Data controls behavior, not branches.** Replace long conditionals on kinds or types with a lookup table, a registry, or a configuration.
- **Keep data separate from logic.** Put constants, rules, content, and tuning values in data structures or configuration files.
- **Model the domain explicitly.** Use typed data classes or records. Do not use loose maps, tuples, or parallel arrays.
- **Single source of truth.** Define each piece of data one time.
- **Use evidence to set priorities.** Churn, file size, duplication, bug history, and test coverage show where to start.

### T3: Readability (changed for this project: function length is a guideline)

- Names tell *what* and *why*, not *how*. Example: `calculateDamage`, not `doCalc2`.
- Each function does one thing. Guideline: 30 lines. A longer function needs a reason. Refer to section 3, "Long functions".
- Shallow nesting: use early returns and guard clauses. Limit: 4 levels.
- No magic numbers or strings: use named constants or data.
- Comments tell *why*. The code tells *what*. Remove commented-out code.
- Formatting and patterns are consistent across the codebase.

### T4: Maintainability

- **DRY, but not too early.** Extract duplicate code at the third occurrence, or when the copies must change together.
- **Small units.** Limit: files of 500 lines or fewer, without the `#[cfg(test)]` module. Later target: 300.
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

- **Extension points** exist where variation is expected: interfaces, registries, plugin hooks, and configuration.
- To add a new variant, you **add a file or a data entry**. You do not edit many existing files.
- Use composition, not inheritance, where possible.
- **YAGNI:** add extension points only where the plan or the history shows real variation.

### T7: Error handling

- Each error message says what failed, with which input, and what the user can do.
- Errors keep their cause. Do not replace a detailed error with a general error.
- One location per layer converts low-level errors into domain errors.
- Expected failures (bad input, missing file) use the normal error type of the language, not a crash.

## 3. Limits

| Limit | Value | Kind |
|---|---|---|
| Jumps of indirection to understand one behavior | 3 | hard |
| Lines in one function | 30 | guideline: longer only with a verified reason |
| Levels of nesting | 4 | hard |
| Lines in one file, without the `#[cfg(test)]` module | 500 (later target: 300) | hard |

### Long functions

1. The refactorer gives each function of more than 30 lines a reason comment: `// Long function: <reason>.`
2. The reviewer checks the reason. If the reason is valid, the reviewer reports it as verified. If not, the reviewer reports a finding with a proposal to divide the function.
3. The review action changes the comment of each verified function to: `// Long function (reason verified, phase <N>): <reason>.`
4. Later reviews treat a verified function as acceptable. They check it again only in these cases:
   - An architecture rewrite: a plan step or a phase whose stated purpose is to restructure the module of the function.
   - The function becomes longer than it was at the verification.
5. A function of more than 100 lines also triggers `clippy::too_many_lines`. Put the reason in `#[expect(clippy::too_many_lines, reason = "...")]`.

## 4. Priority

If targets conflict, use this order:

1. **Correctness:** the behavior stays the same. Tests prove it.
2. **Audience test:** the code passes the A1 audience test.
3. **Readability:** an experienced reader understands the code quickly.
4. **Maintainability, modularity, and error handling:** changes stay local, and failures are explicit.
5. **Extensibility:** only where variation is real.

A simpler design that is slightly less "pure" is better than a textbook pattern that the audience cannot follow.

## 5. Project rules

1. All system paths go through `studio/src/core/paths.rs` (Rust) or `scripts/lib/paths.sh` (bash). New code never hard-codes a path.
2. Only one location runs app reload commands (`pkill`, `i3-msg`, and others).
3. Changed shell scripts pass `shellcheck`. Changed Rust code passes `cargo clippy --all-targets -- -D warnings` with the groups pedantic, style, and correctness (ratchet: refer to GOAL.md, criterion 7).
4. Rust documentation follows the rustdoc style strictly. Refer to the project-conventions skill, section "Documentation style".
