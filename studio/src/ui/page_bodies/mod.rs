//! Drawing the bodies of pages and tabs, one file or folder each.
//!
//! - `settings/`: the Settings page (F3), one file per tab.
//! - `fonts_column.rs`: the fonts column of the Main page.
//! - `fonts_download.rs`: the Download fonts page (F2).
//! - `tournament_tab.rs`: the Tournament tab of the Fonts page.

/// The fonts column of the Main page.
pub mod fonts_column;
/// The Download fonts page.
pub mod fonts_download;
/// The details panel of the Download fonts page.
mod fonts_download_details;
/// The preview panel of the Download fonts page.
mod fonts_download_preview;
/// The font roles list of the fonts column.
mod fonts_roles;
/// The row drawing that several pages share.
mod rows;
/// The Settings page.
pub mod settings;
/// The Tournament tab of the Fonts page.
pub mod tournament_tab;
