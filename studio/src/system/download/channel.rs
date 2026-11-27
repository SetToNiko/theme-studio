//! The channel between the background threads and the Download page.

use std::sync::mpsc::{Receiver, Sender, channel};

use super::types::Msg;

/// Sends a message to the page. The page may be closed already, so that the
/// receiver is gone; then nobody needs the message and the error is ignored.
pub(in crate::system) fn notify(tx: &Sender<Msg>, msg: Msg) {
    // Ignored on purpose: a closed page means nobody waits for the answer.
    let _ = tx.send(msg);
}

/// A channel for the page's background work.
pub fn channel_pair() -> (Sender<Msg>, Receiver<Msg>) {
    channel()
}
