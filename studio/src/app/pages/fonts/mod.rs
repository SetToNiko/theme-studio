//! The Fonts page of `App`.
//!
//! - `fonts_dl_actions.rs`: the Download fonts view.
//! - `fonts_install_actions.rs`: install and remove fonts, the Nerd Font offer.
//! - `fonts_preview_actions.rs`: the font preview.
//! - `tour_actions.rs`: the font tournament, its input.
//! - `tour_rounds.rs`: the tournament rounds.
//! - `prefetch_actions.rs`: background prefetch of fonts.

/// The Download fonts view.
mod fonts_dl_actions;
/// Install and remove fonts.
mod fonts_install_actions;
/// The font preview.
mod fonts_preview_actions;
/// Background prefetch of fonts.
mod prefetch_actions;
/// Input of the font tournament.
mod tour_actions;
/// The tournament rounds.
mod tour_rounds;
