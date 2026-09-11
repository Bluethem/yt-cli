use std::env;
use std::sync::atomic::{AtomicBool, Ordering};

static FORCE_NO_COLOR: AtomicBool = AtomicBool::new(false);

const GREEN: &str = "\x1b[32m";
const RESET: &str = "\x1b[0m";

pub fn set_no_color(value: bool) {
    FORCE_NO_COLOR.store(value, Ordering::Relaxed);
}

pub fn color_enabled() -> bool {
    if FORCE_NO_COLOR.load(Ordering::Relaxed) {
        return false;
    }
    if env::var_os("NO_COLOR").is_some() {
        return false;
    }
    true
}

pub fn local_marker() -> &'static str {
    "[↓]"
}

pub fn stream_marker() -> &'static str {
    "[~]"
}

/// Format title — uploader with mode marker (and optional green for local).
pub fn format_title_uploader(title: &str, uploader: &str, is_local: bool) -> String {
    let marker = if is_local {
        local_marker()
    } else {
        stream_marker()
    };
    let body = format!("{marker} {title} — {uploader}");
    if is_local && color_enabled() {
        format!("{GREEN}{body}{RESET}")
    } else {
        body
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_and_no_color() {
        set_no_color(true);
        let local = format_title_uploader("T", "U", true);
        assert!(local.contains("[↓]"));
        assert!(!local.contains('\x1b'));
        let stream = format_title_uploader("T", "U", false);
        assert!(stream.contains("[~]"));
        set_no_color(false);
    }
}
