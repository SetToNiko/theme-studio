//! The small types that name a preview pane and a preview mode.

/// One of the three hidden programs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaneId {
    Nvim,
    Shell,
    Yazi,
}

/// What the preview pane shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Desktop,
    Swatches,
}
