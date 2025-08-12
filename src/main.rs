#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

use chrono::{DateTime, Utc};
use rdev::{grab, Event, EventType, Key};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use xcap::Monitor;

const TARGET_DIR: &str = "screens";

fn main() -> io::Result<()> {
    // Resolve a single absolute screenshots directory once
    let screens_dir = resolve_screens_dir()?;
    fs::create_dir_all(&screens_dir).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "Failed to create screenshots directory '{}': {e}",
                screens_dir.display()
            ),
        )
    })?;

    // rdev requires a 'static closure; share the path via Arc
    let screens_dir = Arc::new(screens_dir);

    if let Err(err) = grab({
        let screens_dir = Arc::clone(&screens_dir);
        move |e| callback(e, &screens_dir)
    }) {
        eprintln!("Failed to start global keyboard hook: {err:?}");
    }

    Ok(())
}

/// Returns an absolute path to the screenshots directory.
/// Uses first CLI arg if provided, otherwise defaults to TARGET_DIR under current dir.
fn resolve_screens_dir() -> io::Result<PathBuf> {
    let arg_dir = env::args().nth(1);
    let base = env::current_dir()
        .map_err(|e| io::Error::new(e.kind(), format!("Failed to get current directory: {e}")))?;
    let dir = arg_dir.unwrap_or_else(|| TARGET_DIR.to_string());
    Ok(base.join(dir))
}

fn ensure_dir_exists(dir: &Path) -> io::Result<()> {
    if !dir.is_dir() {
        fs::create_dir_all(dir).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("Failed to create directory '{}': {e}", dir.display()),
            )
        })?;
    }
    Ok(())
}

fn callback(event: Event, screens_dir: &Path) -> Option<Event> {
    if is_printscreen(&event) {
        if let Err(err) = ensure_dir_exists(screens_dir) {
            eprintln!("{err}");
            // swallow the event anyway so we don't spam
            return None;
        }
        if let Err(err) = make_screens(screens_dir) {
            eprintln!("{err}");
        }
        // swallow the PrintScreen key so it doesn't propagate
        return None;
    }
    Some(event)
}

fn make_screens(screens_dir: &Path) -> io::Result<()> {
    let monitors = Monitor::all().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to enumerate monitors: {e}"),
        )
    })?;

    if monitors.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No monitors detected for capture.",
        ));
    }

    let now: DateTime<Utc> = Utc::now();
    let ts = now.format("%Y-%m-%d_%H-%M-%S%.3f"); // sortable and ms precision

    for (idx, monitor) in monitors.into_iter().enumerate() {
        let image = monitor.capture_image().map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to capture image from monitor #{idx}: {e}"),
            )
        })?;

        let name = monitor
            .name()
            .ok()
            .and_then(|s| Some(s.trim().to_string()))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("monitor{idx}"));

        let filename = format!("{}-{}-{}.png", ts, idx, normalized(&name));
        let full_path = screens_dir.join(filename);

        // xcap's image.save returns image::ImageResult<()>, convert to io::Result
        image.save(&full_path).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Failed to save screenshot to '{}': {e}",
                    full_path.display()
                ),
            )
        })?;
    }

    Ok(())
}

fn normalized(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| {
            let c = if c.is_ascii() { c } else { '_' };
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
                ' ' => '_',
                _ => '_',
            }
        })
        .collect();

    // collapse multiple underscores
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

#[cfg(not(target_os = "macos"))]
fn is_printscreen(event: &Event) -> bool {
    matches!(event.event_type, EventType::KeyPress(Key::PrintScreen))
}

#[cfg(target_os = "macos")]
fn is_printscreen(event: &Event) -> bool {
    // macOS has no PrintScreen; map to F9 by convention (adjust if you want a different hotkey)
    matches!(event.event_type, EventType::KeyPress(Key::F9))
}
