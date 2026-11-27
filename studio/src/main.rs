//! Theme Studio: the binary. It calls [`theme_studio::cli::run`].

use anyhow::Result;

/// Starts the studio, or does the job that the command line names.
///
/// # Errors
/// Fails when [`theme_studio::cli::run`] fails.
fn main() -> Result<()> {
    theme_studio::cli::run()
}
