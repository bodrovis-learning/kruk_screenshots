#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use xcap::image::{ImageFormat, RgbaImage};
use xcap::Monitor;

#[cfg(target_os = "windows")]
use std::ptr::null_mut;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, TranslateMessage, MSG,
};

#[cfg(target_os = "macos")]
#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn RunCurrentEventLoop(timeout: f64) -> i32;
}

const TARGET_DIR: &str = "screens";

fn main() -> io::Result<()> {
    let screens_dir = resolve_screens_dir()?;
    ensure_dir_exists(&screens_dir)?;

    let hotkey = screenshot_hotkey();

    let manager = GlobalHotKeyManager::new()
        .map_err(|e| io::Error::other(format!("Failed to create hotkey manager: {e}")))?;

    manager
        .register(hotkey)
        .map_err(|e| io::Error::other(format!("Failed to register hotkey: {e}")))?;

    // Keep the manager alive for the entire event loop.
    run_event_loop(&screens_dir, hotkey)
}

// Windows requires a native message loop on the thread
// where GlobalHotKeyManager was created.
#[cfg(target_os = "windows")]
fn run_event_loop(screens_dir: &Path, hotkey: HotKey) -> io::Result<()> {
    let receiver = GlobalHotKeyEvent::receiver();

    // SAFETY: MSG is a plain Win32 structure, and these functions
    // are called on the thread that owns the hotkey manager.
    unsafe {
        let mut msg: MSG = std::mem::zeroed();

        loop {
            let ret = GetMessageW(&raw mut msg, null_mut(), 0, 0);

            if ret == -1 {
                return Err(io::Error::last_os_error());
            }

            if ret == 0 {
                break;
            }

            TranslateMessage(&raw const msg);
            DispatchMessageW(&raw const msg);

            while let Ok(event) = receiver.try_recv() {
                handle_hotkey_event(event, hotkey, screens_dir);
            }
        }
    }

    Ok(())
}

// Linux X11 receives hotkey events through a channel.
#[cfg(target_os = "linux")]
fn run_event_loop(screens_dir: &Path, hotkey: HotKey) -> io::Result<()> {
    let receiver = GlobalHotKeyEvent::receiver();

    loop {
        let event = receiver
            .recv()
            .map_err(|e| io::Error::other(format!("Failed to receive hotkey event: {e}")))?;

        handle_hotkey_event(event, hotkey, screens_dir);
    }
}

// Carbon hotkey events require a native event loop on the main thread.
#[cfg(target_os = "macos")]
fn run_event_loop(screens_dir: &Path, hotkey: HotKey) -> io::Result<()> {
    let receiver = GlobalHotKeyEvent::receiver();

    loop {
        // SAFETY: RunCurrentEventLoop is invoked on the main thread,
        // where GlobalHotKeyManager was initialized.
        let status = unsafe { RunCurrentEventLoop(0.1) };

        match status {
            0 | -9875 => {} // Success or event loop timeout.
            -9876 => break, // Event loop was asked to quit.
            _ => {
                return Err(io::Error::other(format!(
                    "RunCurrentEventLoop failed: OSStatus {status}"
                )));
            }
        }

        while let Ok(event) = receiver.try_recv() {
            handle_hotkey_event(event, hotkey, screens_dir);
        }
    }

    Ok(())
}

// Screenshot errors are recoverable and must not stop the event loop.
fn handle_hotkey_event(event: GlobalHotKeyEvent, hotkey: HotKey, screens_dir: &Path) {
    if is_matching_press(event, hotkey) {
        if let Err(err) = capture_all_monitors(screens_dir) {
            eprintln!("Screenshot failed: {err}");
        }
    }
}

fn is_matching_press(event: GlobalHotKeyEvent, hotkey: HotKey) -> bool {
    event.id == hotkey.id() && event.state == HotKeyState::Pressed
}

#[cfg(not(target_os = "macos"))]
fn screenshot_hotkey() -> HotKey {
    HotKey::new(Some(Modifiers::CONTROL), Code::PrintScreen)
}

// Standard Mac keyboards do not have a Print Screen key.
#[cfg(target_os = "macos")]
fn screenshot_hotkey() -> HotKey {
    HotKey::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyS)
}

/// Resolves the screenshots directory relative to the working directory,
/// unless an absolute path is supplied as the first CLI argument.
fn resolve_screens_dir() -> io::Result<PathBuf> {
    let arg_dir = env::args().nth(1);
    let base = env::current_dir()?;
    let dir = arg_dir.unwrap_or_else(|| TARGET_DIR.to_string());

    Ok(base.join(dir))
}

fn ensure_dir_exists(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to create directory '{}': {e}", dir.display()),
        )
    })
}

/// Captures every available monitor, continuing if an individual
/// monitor fails. Returns combined errors after processing all monitors.
fn capture_all_monitors(screens_dir: &Path) -> io::Result<()> {
    ensure_dir_exists(screens_dir)?;

    let monitors = Monitor::all()
        .map_err(|e| io::Error::other(format!("Failed to enumerate monitors: {e}")))?;

    if monitors.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No monitors detected for capture",
        ));
    }

    let timestamp = filename_timestamp()?;
    let mut errors = Vec::new();

    for (idx, monitor) in monitors.into_iter().enumerate() {
        let monitor_name = monitor
            .name()
            .ok()
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| format!("monitor{idx}"));

        let normalized_name = normalized(&monitor_name);
        let safe_name = if normalized_name.is_empty() {
            format!("monitor{idx}")
        } else {
            normalized_name
        };

        let image = match monitor.capture_image() {
            Ok(image) => image,
            Err(err) => {
                errors.push(format!("Monitor #{idx} ({monitor_name}): {err}"));
                continue;
            }
        };

        let filename_stem = format!("{timestamp}-{idx}-{safe_name}");

        if let Err(err) = save_png_unique(&image, screens_dir, &filename_stem) {
            errors.push(format!("Monitor #{idx} ({monitor_name}): {err}"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(io::Error::other(errors.join("; ")))
    }
}

/// Saves a PNG without ever overwriting an existing screenshot.
///
/// If the filename already exists, appends an increasing numeric suffix.
/// Removes partially written files when encoding or writing fails.
fn save_png_unique(
    image: &RgbaImage,
    screens_dir: &Path,
    filename_stem: &str,
) -> io::Result<PathBuf> {
    let mut suffix = 0_u64;

    loop {
        let filename = if suffix == 0 {
            format!("{filename_stem}.png")
        } else {
            format!("{filename_stem}-{suffix}.png")
        };

        let full_path = screens_dir.join(filename);

        let file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&full_path)
        {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                suffix = suffix
                    .checked_add(1)
                    .ok_or_else(|| io::Error::other("Screenshot filename counter overflow"))?;
                continue;
            }
            Err(err) => {
                return Err(io::Error::new(
                    err.kind(),
                    format!("Failed to create '{}': {err}", full_path.display()),
                ));
            }
        };

        let result = (|| -> io::Result<()> {
            let mut writer = BufWriter::new(file);

            image
                .write_to(&mut writer, ImageFormat::Png)
                .map_err(io::Error::other)?;

            writer.flush()
        })();

        if let Err(err) = result {
            // Best-effort cleanup of an incomplete PNG.
            let _ = fs::remove_file(&full_path);

            return Err(io::Error::new(
                err.kind(),
                format!("Failed to save '{}': {err}", full_path.display()),
            ));
        }

        return Ok(full_path);
    }
}

/// Returns a timestamp formatted as Unix seconds and milliseconds.
/// Uniqueness is guaranteed by `save_png_unique`, not by the timestamp.
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

/// Converts a monitor name to a filesystem-friendly string.
fn normalized(s: &str) -> String {
    let mut out = String::new();
    let mut previous_underscore = false;

    for c in s.chars() {
        let safe = match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
            _ => '_',
        };

        if safe == '_' && previous_underscore {
            continue;
        }

        previous_underscore = safe == '_';
        out.push(safe);
    }

    out.trim_matches('_').to_string()
}
