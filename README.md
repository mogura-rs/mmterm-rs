# mmterm (Rust)

A terminal-based viewer for PDB protein structures, implemented in Rust.
This is a port of the original [mmterm](https://github.com/jgreener64/mmterm) Python tool by [jgreener64](https://github.com/jgreener64).

It uses Braille characters to render 3D protein structures directly in your terminal.

## Features

- **Terminal Rendering:** View protein structures in any terminal that supports Unicode (Braille patterns).
- **Interactive Controls:** Rotate, translate, and zoom the structure using keyboard shortcuts.
- **Multiple Formats:** Supports `.pdb` and `.mmcif` (and `.cif`) files.
- **Filtering:** Select specific chains or models to view.
- **Performance:** Written in Rust for efficiency.

## Installation

### From Source

Ensure you have [Rust and Cargo installed](https://rustup.rs/).

```bash
git clone https://github.com/yourusername/mmterm-rs
cd mmterm-rs
cargo install --path .
```

This will install the `mmterm` binary to your Cargo bin directory (usually `~/.cargo/bin`).

## Usage

```bash
mmterm <pdb_file> [options]
```

### Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--size` | `-s` | Size of the viewing box (10-400) | 100.0 |
| `--model` | `-m` | Initial model to show (1-based) | 1 |
| `--chain` | `-c` | Filter to show only a specific chain ID | (All) |
| `--format` | `-f` | Force input format (`pdb`, `mmcif`) | (Auto) |
| `--help` | `-h` | Show help message | |

### Examples

View a file:
```bash
mmterm 1CRN.pdb
```

View only chain A:
```bash
mmterm 1CRN.pdb -c A
```

Force mmCIF format:
```bash
mmterm structure.cif -f mmcif
```

### Controls

| Key | Action |
|-----|--------|
| **W / A / S / D** | Rotate the structure |
| **T / F / G / H** | Translate (move) the view |
| **I / O** | Zoom In / Out |
| **U** | Toggle Auto-Spin |
| **P** | Cycle through Models (if multiple exist) |
| **Q / Esc** | Quit |

## References & Credits

- **Original Implementation:** [mmterm](https://github.com/jgreener64/mmterm) by Joe Greener.
- **PDB Parsing:** Uses the [pdbtbx](https://crates.io/crates/pdbtbx) crate.
- **TUI/Input:** Uses [crossterm](https://crates.io/crates/crossterm).
- **Math:** Uses [glam](https://crates.io/crates/glam) for 3D transformations.

## License

MIT
