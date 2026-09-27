//! Advertise Codex-compatible screen detection to a containing Herdr pane.
//!
//! Herdr reads the foreground process's launch environment through procfs or
//! KERN_PROCARGS2. Re-exec before starting threads makes the hint visible there
//! while preserving the executable, arguments, PID, terminal, and branding.

use std::os::unix::process::CommandExt;
use std::process::Command;

pub(super) fn ensure_process_hint() -> std::io::Result<()> {
    if std::env::var_os("HERDR_AGENT").is_some()
        || std::env::var_os("HERDR_PANE_ID").is_none_or(|pane| pane.is_empty())
    {
        return Ok(());
    }

    let mut arguments = std::env::args_os();
    let executable = std::env::current_exe()?;
    let mut command = Command::new(&executable);
    if let Some(arg0) = arguments.next() {
        command.arg0(arg0);
    }
    Err(command.args(arguments).env("HERDR_AGENT", "codex").exec())
}

#[cfg(test)]
#[path = "herdr_tests.rs"]
mod tests;
