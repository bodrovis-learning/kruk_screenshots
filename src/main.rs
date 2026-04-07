#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use xcap::Monitor;

#[cfg(target_os = "windows")]
use std::ptr::null_mut;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, TranslateMessage, MSG,
};

const TARGET_DIR: &str = "screens";

fn main() -> io::Result<()> {
    let screens_dir = resolve_screens_dir()?;
    ensure_dir_exists(&screens_dir)?;

    let hotkey = screenshot_hotkey();

    let manager = GlobalHotKeyManager::new()
        .map_err(|e| io::Error::other(format!("Failed to create global hotkey manager: {e}")))?;

    manager
        .register(hotkey)
        .map_err(|e| io::Error::other(format!("Failed to register global hotkey: {e}")))?;

    run_event_loop(&screens_dir, hotkey)
}

#[cfg(target_os = "windows")]
fn run_event_loop(screens_dir: &Path, hotkey: HotKey) -> io::Result<()> {
    let receiver = GlobalHotKeyEvent::receiver();

    // SAFETY:
    // Windows global hotkeys require a native message loop on the same thread
    // where the hotkey manager was created.
    unsafe {
        let mut msg: MSG = std::mem::zeroed();

        loop {
            let ret = GetMessageW(&raw mut msg, null_mut(), 0, 0);

            if ret == -1 {
                return Err(io::Error::other("GetMessageW failed"));
            }

            if ret == 0 {
                break;
            }

            TranslateMessage(&raw const msg);
            DispatchMessageW(&raw const msg);

            while let Ok(event) = receiver.try_recv() {
                if is_matching_press(event, hotkey) {
                    capture_all_monitors(screens_dir)?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn run_event_loop(screens_dir: &Path, hotkey: HotKey) -> io::Result<()> {
    let receiver = GlobalHotKeyEvent::receiver();

    loop {
        let event = receiver
            .recv()
            .map_err(|e| io::Error::other(format!("Failed to receive global hotkey event: {e}")))?;

        if is_matching_press(event, hotkey) {
            capture_all_monitors(screens_dir)?;
        }
    }
}

fn is_matching_press(event: GlobalHotKeyEvent, hotkey: HotKey) -> bool {
    event.id == hotkey.id() && event.state == HotKeyState::Pressed
}

fn screenshot_hotkey() -> HotKey {
    HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyS)
}

/// Resolves the absolute screenshots directory path.
/// Uses the first CLI argument when provided; otherwise falls back to
/// `TARGET_DIR` inside the current working directory.
fn resolve_screens_dir() -> io::Result<PathBuf> {
    let arg_dir = env::args().nth(1);
    let base = env::current_dir()
        .map_err(|e| io::Error::new(e.kind(), format!("Failed to get current directory: {e}")))?;
    let dir = arg_dir.unwrap_or_else(|| TARGET_DIR.to_string());
    Ok(base.join(dir))
}

fn ensure_dir_exists(dir: &Path) -> io::Result<()> {
    if dir.is_dir() {
        return Ok(());
    }

    fs::create_dir_all(dir).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to create directory '{}': {e}", dir.display()),
        )
    })
}

/// Captures one screenshot per detected monitor and saves the files into `screens_dir`.
fn capture_all_monitors(screens_dir: &Path) -> io::Result<()> {
    ensure_dir_exists(screens_dir)?;

    let monitors = Monitor::all()
        .map_err(|e| io::Error::other(format!("Failed to enumerate monitors: {e}")))?;

    if monitors.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No monitors detected for capture.",
        ));
    }

    let timestamp = filename_timestamp()?;

    for (idx, monitor) in monitors.into_iter().enumerate() {
        let image = monitor.capture_image().map_err(|e| {
            io::Error::other(format!("Failed to capture image from monitor #{idx}: {e}"))
        })?;

        let monitor_name = monitor
            .name()
            .ok()
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| format!("monitor{idx}"));

        let filename = format!("{timestamp}-{idx}-{}.png", normalized(&monitor_name));
        let full_path = screens_dir.join(filename);

        image.save(&full_path).map_err(|e| {
            io::Error::other(format!(
                "Failed to save screenshot to '{}': {e}",
                full_path.display()
            ))
        })?;
    }

    Ok(())
}

/// Returns a Unix timestamp formatted as "seconds-milliseconds".
/// The value is monotonically increasing (assuming system clock is stable),
/// making it suitable for sortable and collision-resistant filenames.
fn filename_timestamp() -> io::Result<String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| io::Error::other(format!("System clock error: {e}")))?;

    Ok(format!(
        "{}-{:03}",
        duration.as_secs(),
        duration.subsec_millis()
    ))
}

/// Converts an arbitrary monitor name into a filesystem-friendly string.
fn normalized(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| {
            let c = if c.is_ascii() { c } else { '_' };
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
                _ => '_',
            }
        })
        .collect();

    while out.contains("__") {
        out = out.replace("__", "_");
    }

    out.trim_matches('_').to_string()
}
