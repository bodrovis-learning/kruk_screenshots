#![warn(clippy::all, clippy::pedantic)]

use chrono::{DateTime, Utc};
use rdev::{grab, Event, EventType, Key};
use std::env;
use std::fs;
use std::path::PathBuf;
use xcap::Monitor;

const TARGET_DIR: &str = "screens";

fn main() -> std::io::Result<()> {
    let screens_dir = get_screens_dir();
    let mut path = env::current_dir()?;
    path.push(&screens_dir);

    fs::create_dir_all(path)?;

    if let Err(error) = grab(move |e| callback(e, &screens_dir)) {
        println!("Error: {error:?}");
    }

    Ok(())
}

fn get_screens_dir() -> String {
    let args: Vec<String> = env::args().collect();

    let screens_dir = args.get(1).unwrap_or(&TARGET_DIR.to_string()).to_string();

    screens_dir
}


fn repair_screens_dir() -> bool {
    let screens_dir = get_screens_dir();

    match fs::create_dir(&screens_dir) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("Ошибка: {}", e);
            false
        },
    }
}

fn check_screens_dir(dir_path: &str) -> bool {
    let full_path = PathBuf::from(dir_path);

    std::path::Path::new(&full_path).is_dir()
}

fn callback(event: Event, screens_dir: &str) -> Option<Event> {
    if is_printscreen(&event) {
        if check_screens_dir(screens_dir) == false {
            repair_screens_dir();
        }
        make_screen(screens_dir);

        return None;
    }
    Some(event)
}



fn make_screen(screens_dir: &str) {
    let monitors = Monitor::all().unwrap();

    for monitor in monitors {
        let image = monitor.capture_image().unwrap();

        let now: DateTime<Utc> = Utc::now();

        let monitor_name_result = monitor.name();
        let name = monitor_name_result.as_deref().unwrap_or("unknown");
        let filename = format!(
            "{}-{}.png",
            now.format("%d-%m-%Y_%H_%M_%S"),
            normalized(name)
        );
        let mut full_path = PathBuf::from(screens_dir);
        full_path.push(filename);

        image.save(full_path).unwrap();
    }
}

fn normalized(filename: &str) -> String {
    filename
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .collect()
}

#[cfg(not(target_os = "macos"))]
fn is_printscreen(event: &Event) -> bool {
    matches!(event.event_type, EventType::KeyPress(Key::PrintScreen))
}

#[cfg(target_os = "macos")]
fn is_printscreen(event: &Event) -> bool {
    matches!(event.event_type, EventType::KeyPress(Key::F9))
}