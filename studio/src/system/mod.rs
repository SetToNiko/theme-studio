//! The parts of the studio that talk to the system outside it: other
//! programs, the network, and the terminal.
//! - `desktop.rs`: reads the real i3, rofi and i3blocks configs for the
//!   preview's model desktop.
//! - `download/`: downloads fonts (curl, tar, fc-cache) for the Download page.
//! - `prefetch.rs`: fetches font previews ahead of time, in background threads.
//! - `out.rs`: the terminal output of each frame.
//! - `switch.rs`: runs `theme-switch` and `font-switch` (the only place that does).

/// Reads the real i3, rofi and i3blocks configs for the preview.
pub mod desktop;
/// Downloads fonts (curl, tar, fc-cache) for the Download page.
pub mod download;
/// The terminal output of each frame.
pub mod out;
/// Fetches font previews ahead of time, in background threads.
pub mod prefetch;
/// Runs `theme-switch` and `font-switch`.
pub mod switch;
