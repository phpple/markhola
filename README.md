# MarkHola

MarkHola is a lightweight desktop Markdown reader and editor built with Rust for Apple Silicon Macs.

## Current Version

- `v0.7.2`

## Features

- Readonly and writable modes with `Command + /` mode switching
- Open local `.md` and `.markdown` files
- Open and keep multiple Markdown documents in one window
- Export the current document to PDF from `File > Export > PDF`
- Export the current document to HTML from `File > Export > HTML`
- Print the current document from `File > Print`
- Load the app shell theme from an editable `themes/default/layout.css` file
- Save the current document with `Command + S`
- Save the current document to another path with `File > Save As`
- Open the bundled documentation from `Help > Documentation`
- Render headings, links, images, tables, lists, blockquotes, and code blocks
- Syntax-highlighted fenced code blocks in readonly mode
- Improved mainstream language highlight coverage for fenced code blocks
- Mathematical expressions in readonly mode, including inline math, `$$...$$`, and fenced `math` blocks
- Code block line numbers and hover language badges in readonly mode
- Render Mermaid fenced code blocks in readonly mode
- Writable editor line numbers
- Writable editor shortcuts:
  - `Command + A` select all
  - `Command + C / V / X` copy, paste, and cut
  - `Command + Z / R` undo and redo
  - `Ctrl + A / E` move to line start and line end
  - `Tab / Shift + Tab` indent and outdent, including multi-line selections
- `Command + W` close the current document tab
- Drag and drop files into the window
- Open Markdown files from Finder on macOS
- Open external links in the default browser
- macOS app bundle and DMG packaging

## Platform

- macOS on Apple Silicon

## Tech Stack

- Rust

## Third-Party Libraries

- `block2`
- `chardetng`
- `encoding_rs`
- `icns`
- `lopdf`
- `objc2`
- `objc2-app-kit`
- `objc2-core-foundation`
- `objc2-foundation`
- `objc2-web-kit`
- `open`
- `pulldown-cmark`
- `rfd`
- `serde`
- `serde_json`
- `syntect`
- `tao`
- `url`
- `wry`

## Development

Run tests:

```bash
cargo test
```

Run release regression checks:

```bash
./scripts/release_regression.sh
```

Run release regression checks with packaging:

```bash
./scripts/release_regression.sh --with-package
```

Build the app:

```bash
cargo build
```

Create the macOS app bundle and DMG:

```bash
./scripts/package_dmg.sh
```

## Project Structure

- `src/`: desktop app source code
- `src/bin/make_icns.rs`: macOS icon generation helper
- `assets/`: logo and icon sources
- `examples/`: sample Markdown files for manual verification
- `scripts/`: packaging scripts
- `themes/`: directly editable app theme files
- `assets/help/`: bundled in-app help markdown files

## GitHub

<https://github.com/huimang/markhola>
