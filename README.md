# Canopy

<p align="center">
<img width="128" height="128" alt="canopy" src="https://github.com/user-attachments/assets/8b8fb294-f8f9-4f67-9213-047e83352bc7" />
<p/>


A fast, interactive disk space visualizer using treemaps. Built with Rust and egui.

<img style="border-radius: 50;" width="1024" height="736" alt="image" src="https://github.com/user-attachments/assets/5225f15d-6fa1-4953-9e75-e6b515e4b8de" />

## Features

- **Treemap visualization** — see where your disk space goes at a glance
- **Interactive navigation** — click to zoom into directories, right-click to go back
- **Fast scanning** — parallel directory traversal with jwalk and rayon
- **CLI & GUI** — pass a path as argument or pick a folder with the file dialog

## Installation

### From source

```bash
git clone https://github.com/antoskuu/canopy.git
cd canopy
cargo build --release
```

The binary will be at `target/release/canopy`.

### AppImage (Linux)

```bash
./build-appimage.sh
```

## Usage

```bash
# Scan a specific directory
canopy /path/to/directory

# Launch and pick a folder via dialog
canopy
```

## Tech stack

- [Rust](https://www.rust-lang.org/)
- [eframe/egui](https://github.com/emilk/egui) — immediate mode GUI
- [jwalk](https://github.com/jessegrosjean/jwalk) — parallel filesystem walking
- [rayon](https://github.com/rayon-rs/rayon) — data parallelism
- [rfd](https://github.com/PolyMeilex/rfd) — native file dialogs
- [clap](https://github.com/clap-rs/clap) — CLI argument parsing

## License

[MIT](LICENSE)
