# Screenshot capturer

A lightweight, cross-platform screenshot utility written in Rust. Run it, press a hotkey, and get PNG screenshots of all detected monitors.

## Supported platforms

Prebuilt binaries are available for:

- **Windows:** x64, x86, ARM64
- **macOS:** Intel, Apple Silicon, universal binary
- **Linux:** x64, ARM64

> **Note:** Global hotkeys on Linux currently require X11. Wayland is not supported by the current hotkey implementation.

## Usage

Download the appropriate archive from GitHub Releases, extract it, and run the executable.

Press the screenshot hotkey:

| Platform | Hotkey |
|---|---|
| Windows | `Ctrl+PrintScreen` |
| Linux | `Ctrl+PrintScreen` |
| macOS | `Cmd+Shift+S` |

Screenshots are saved as PNG files in the `screens` directory, created automatically in the current working directory. Each monitor gets its own screenshot with a timestamp in the filename. Existing screenshots are never overwritten.

You can also specify a custom output directory as the first command-line argument:

```bash
kruk_screenshots /path/to/screenshots
```

Keep the application running to capture additional screenshots. On macOS, you may need to grant Screen Recording permission.

## Building from source

Install Rust and Cargo, then run:

```bash
cargo build --release --locked
```

The executable will be available in `target/release/` (with an `.exe` extension on Windows).

### Dependencies

- [global-hotkey](https://github.com/tauri-apps/global-hotkey) — global keyboard shortcuts
- [xcap](https://github.com/nashaofu/xcap) — screen capture
- [windows-sys](https://crates.io/crates/windows-sys) — Windows APIs

## How it was made

**[Watch the tutorial on YouTube](https://youtu.be/gva5UYcHVWM)** (in Russian).

## Windows alternative

Looking for a dedicated Windows screenshot utility? Check out [Kruk Shot](https://github.com/bodrovis-learning/kruk-shot), written in Go and using DXGI Desktop Duplication for capturing desktop applications and fullscreen games.

## Overriding directory name

To override the screenshots directory, simply provide the desired name when running the program:

```
kruk_screenshots custom_dir
```

## Building

```
cargo build --release
```

## License

(c) [Ilya Krukowski](http://bodrovis.tech/), licensed under the [beer-ware license](https://fedoraproject.org/wiki/Licensing/Beerware).
