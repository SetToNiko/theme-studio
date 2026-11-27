//! Reads the blocks of the status bar for the preview, on a background thread.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::system::desktop::{self, BarBlock};

/// Reads the bar's blocks on a thread (the commands can be slow), then
/// asks for a redraw.
pub(super) fn read_bar_in_background(
    bar: Arc<Mutex<Option<Vec<BarBlock>>>>,
    dirty: Arc<AtomicBool>,
    configs: PathBuf,
) {
    std::thread::spawn(move || {
        let blocks = desktop::read_bar(&configs, Duration::from_secs(3));
        *bar.lock().unwrap() = Some(blocks);
        dirty.store(true, Ordering::Relaxed);
    });
}
