#[cfg(windows)]
#[path = "controls_windows.rs"]
mod windows;
#[cfg(windows)]
pub(crate) use windows::*;

#[cfg(not(windows))]
pub(crate) fn suspended() -> bool {
    false
}
