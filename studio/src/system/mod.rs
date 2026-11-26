//! The parts of the studio that talk to the system outside it: other
//! programs, the network, and the terminal.
//! - `desktop.rs`: reads the real i3, rofi and i3blocks configs for the
//!   preview's model desktop.
//! - `download/`: downloads fonts (curl, tar, fc-cache) for the Download page.
//! - `prefetch.rs`: fetches font previews ahead of time, in background threads.
//! - `out.rs`: the terminal output of each frame.

pub mod desktop;
pub mod download;
pub mod out;
pub mod prefetch;
