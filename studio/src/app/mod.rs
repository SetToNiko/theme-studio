//! Editor state and input handling: the `App` struct and its methods.
//!
//! `App` is one struct. Its methods are split by topic over several files
//! (Rust allows more than one `impl App` block, in any file of the crate).
//! Each file's own `//!` says what stays outside it. A key press starts at
//! `input/actions.rs` (`run_action`). Where to look:
//! - `state/`: the data behind `App`: `nav.rs` (where the user is: `Page`, `Focus`, `Overlay`, ...), `palette_editor.rs`, `layout.rs`, `messages.rs` (own structs, not `impl App`)
//! - `input/`: `actions.rs` (`run_action`), `focus.rs`, `mouse.rs`, `overlay.rs` (pop-ups), `colours.rs`
//! - `pages/settings/`: `settings_actions.rs` (the page as a whole), `settings_keymaps_actions.rs`, `settings_history_actions.rs`, `draft_actions.rs` (Settings drafts), `behaviour_actions.rs`, `behaviour_colours_actions.rs`, `behaviour_labels_actions.rs` (Behaviour tab)
//! - `pages/fonts/`: `fonts_dl_actions.rs` (Download fonts), `fonts_preview_actions.rs`, `prefetch_actions.rs`, `tour_actions.rs` and `tour_rounds.rs` (Tournament)
//! - `pages/themes/`: `theme_actions.rs`, `presets_actions.rs`
//! - this folder: `app_state.rs` (the `App` struct), `startup.rs` (make an `App`), `methods/` (core methods: `queries.rs`, `editing.rs`, `sample.rs`, `pointer.rs`, `clipboard.rs`), `run.rs` (the main loop and `Term`), `persist.rs` (save, open, unsaved-changes prompt), `test_support.rs` (test helpers), `tests/` (tests of the above, one file per area)
//!
//! The run loop (`App::run`) does not draw by itself: `cli.rs` passes in `ui::screen::frame::draw`.
//!
//! All `impl App` blocks are in this folder: no other module of the crate adds
//! methods to `App`.

/// The `App` struct: the data of the whole studio.
pub mod app_state;
/// Key and mouse input.
mod input;
/// Core methods of `App`: queries, editing, sample, pointer, clipboard.
mod methods;
/// The `impl App` code of each page.
mod pages;
/// Save, open, and the unsaved-changes prompt.
pub(crate) mod persist;
/// The main loop and the `Term` type.
pub(crate) mod run;
/// Make an `App` and the `Start` choice.
pub(crate) mod startup;
/// The data behind `App`.
pub mod state;
/// Shared set-up helpers for the tests.
#[cfg(test)]
pub(crate) mod test_support;
/// Tests of the app, one file per area.
#[cfg(test)]
mod tests;
