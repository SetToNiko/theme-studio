//! The status line: the page tabs, a message, and the key hints.
//!
//! - `draw.rs`: `draw_status` puts the tabs, the message and the hints side by side.
//! - `context.rs`: the hints for the current control, picked by the focus.
//! - `rows.rs`: one small function per screen that lists the actions to show.
//! - `hint.rs`: one hint from the keys of the active keymap, and the general hints.
//! - `fit.rs`: cuts the hints to the room of the line.

/// The hints for the current control.
mod context;
/// The status line itself.
pub mod draw;
/// The hints that fit in the room of the line.
mod fit;
/// One hint from the active keymap, and the general hints.
mod hint;
/// The actions to show for each screen.
mod rows;
