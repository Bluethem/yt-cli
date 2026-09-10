use crate::{Result, YtcliError};
use std::process::Command;

pub fn which_bin(name: &str) -> Result<()> {
    let found = Command::new("sh")
        .args(["-c", &format!("command -v {name}")])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if found {
        Ok(())
    } else {
        Err(YtcliError::MissingBinary(name.into()))
    }
}
