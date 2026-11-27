//! Downloading fonts (the F2 page). Google fonts are listed by the Fontsource
//! API and downloaded whole from the google/fonts GitHub repo; other
//! Fontsource fonts come from Fontsource (latin + latin-ext); Nerd Fonts come
//! from the latest ryanoasis/nerd-fonts release. Files go to
//! `~/.local/share/fonts/theme-studio/<id>/`, and each download is added to
//! the installed-fonts list that packaging/nix/fonts.nix reads, so a rebuild makes it
//! permanent. Network and archive work goes through curl, tar and fc-cache,
//! in background threads.
//! - `net.rs`: curl calls and the cached JSON answers.
//! - `catalogue.rs`: the font lists of Fontsource and Nerd Fonts.
//! - `install.rs`: the install job (download, unpack, fc-cache) and removal.
//! - `preview.rs`: the preview font cache.
//! - `list.rs`: the installed-fonts list file.
//! - `types.rs`: the types the other files share.
//! - `channel.rs`: the channel and `notify` for the background threads.
//! - `files.rs`: clean-up of leftover files and `~/` paths.

/// The font lists of Fontsource and Nerd Fonts.
pub mod catalogue;
/// The channel between the background threads and the page.
pub mod channel;
/// File helpers: clean-up of leftovers and `~/` paths.
pub mod files;
/// The install job (download, unpack, fc-cache) and removal.
pub mod install;
/// The installed-fonts list file.
pub mod list;
/// Curl calls and the cached JSON answers.
pub mod net;
/// The preview font cache.
pub mod preview;
/// The types that the other files share.
pub mod types;
