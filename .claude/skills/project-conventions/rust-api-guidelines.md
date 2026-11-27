# Rust API Guidelines: the checklist for agents

Source: https://rust-lang.github.io/api-guidelines/ (checklist: https://rust-lang.github.io/api-guidelines/checklist.html). For the full text of an item, read its page on that site.

The setup copies this file into the project, next to the project-conventions skill. Each agent obeys this checklist when it writes, changes, reviews, simplifies, tests, or documents Rust code.

## How to apply the checklist
- Apply each item to each public item that you write or change. In a binary crate, "public" means each item that another module of the crate uses (`pub` or `pub(crate)`).
- An item that has no subject in the code needs no work. Example: the macro items, if the crate has no macros.
- If an item conflicts with a rule of the project-conventions skill, obey the skill and report the conflict. Example: the audience test prefers a plain function over a generic one (C-GENERIC).
- A reviewer gives the item ID in each finding about this checklist. Example: `src/model/color.rs:12 | getter named get_hex | C-GETTER | rename to hex`.

## Naming
- C-CASE: casing obeys RFC 430 (`UpperCamelCase` types and traits, `snake_case` functions and modules, `SCREAMING_SNAKE_CASE` constants).
- C-CONV: ad-hoc conversions use the prefixes `as_` (free, borrowed), `to_` (expensive), and `into_` (consumes self).
- C-GETTER: a getter has the name of the field, with no `get_` prefix.
- C-ITER: methods on collections that make iterators are `iter`, `iter_mut`, and `into_iter`.
- C-ITER-TY: the name of an iterator type agrees with the method that makes it.
- C-FEATURE: Cargo feature names have no placeholder words (no `use-`, no `with-`).
- C-WORD-ORDER: names use the same word order. Example: `ParseIntError` and `ParseFloatError`.

## Interoperability
- C-COMMON-TRAITS: types implement the common traits where they fit: `Copy`, `Clone`, `Eq`, `PartialEq`, `Ord`, `PartialOrd`, `Hash`, `Debug`, `Display`, `Default`.
- C-CONV-TRAITS: conversions use the standard traits `From`, `AsRef`, `AsMut` (and `TryFrom`).
- C-COLLECT: collections implement `FromIterator` and `Extend`.
- C-SERDE: data structures implement Serde's `Serialize` and `Deserialize`, if the crate uses Serde.
- C-SEND-SYNC: types are `Send` and `Sync` where possible.
- C-GOOD-ERR: error types are meaningful: they implement `std::error::Error`, `Debug`, `Display`, `Send`, and `Sync`. `()` is never an error type.
- C-NUM-FMT: binary number types give `UpperHex`, `LowerHex`, `Octal`, and `Binary` formats.
- C-RW-VALUE: generic reader and writer functions take `R: Read` and `W: Write` by value.

## Macros
- C-EVOCATIVE: the input syntax of a macro looks like its output.
- C-MACRO-ATTR: macros work with attributes.
- C-ANYWHERE: item macros work at each place where an item is permitted.
- C-MACRO-VIS: item macros accept visibility specifiers.
- C-MACRO-TY: type fragments are flexible.

## Documentation
- C-CRATE-DOC: the crate documentation is complete and has examples.
- C-EXAMPLE: each public item has a rustdoc example.
- C-QUESTION-MARK: examples use `?`, not `try!` and not `unwrap`.
- C-FAILURE: function documentation gives the error, panic, and safety conditions (`# Errors`, `# Panics`, `# Safety`).
- C-LINK: the text has links to the related items (intra-doc links).
- C-METADATA: `Cargo.toml` has the common metadata: authors, description, license, homepage, documentation, repository, keywords, categories.
- C-RELNOTES: release notes record each significant change.
- C-HIDDEN: rustdoc does not show implementation details that do not help (`#[doc(hidden)]`, private fields).

## Predictability
- C-SMART-PTR: smart pointers do not add inherent methods.
- C-CONV-SPECIFIC: a conversion is on the most specific type that it involves.
- C-METHOD: a function with a clear receiver is a method.
- C-NO-OUT: functions do not use out-parameters. They return values.
- C-OVERLOAD: operator overloads do what the operator usually does.
- C-DEREF: only smart pointers implement `Deref` and `DerefMut`.
- C-CTOR: constructors are static, inherent methods (`new`, `with_...`).

## Flexibility
- C-INTERMEDIATE: functions give their intermediate results, so the caller does not do the work again.
- C-CALLER-CONTROL: the caller decides where to copy data and where to put it.
- C-GENERIC: functions make few assumptions about their parameters, because they use generics.
- C-OBJECT: a trait is object-safe if a trait object of it can be useful.

## Type safety
- C-NEWTYPE: newtypes give static distinctions.
- C-CUSTOM-TYPE: arguments give their meaning through types, not through `bool` or `Option`.
- C-BITFLAG: a set of flags uses `bitflags`, not an enum.
- C-BUILDER: builders make complex values.

## Dependability
- C-VALIDATE: functions validate their arguments.
- C-DTOR-FAIL: destructors never fail.
- C-DTOR-BLOCK: a destructor that can block has an alternative.

## Debuggability
- C-DEBUG: each public type implements `Debug`.
- C-DEBUG-NONEMPTY: the `Debug` output is never empty.

## Future proofing
- C-SEALED: sealed traits stop implementations in other crates.
- C-STRUCT-PRIVATE: structs have private fields.
- C-NEWTYPE-HIDE: newtypes hide the implementation details.
- C-STRUCT-BOUNDS: data structures do not repeat the bounds of derived traits.

## Necessities
- C-STABLE: the public dependencies of a stable crate are stable.
- C-PERMISSIVE: the crate and its dependencies have a permissive license.
