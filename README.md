# MarkHola

MarkHola is a lightweight desktop Markdown reader and editor built with Rust for Apple Silicon Macs.

## Current Version

- `v0.6.2`

## Features

- Readonly and writable modes with `Command + /` mode switching
- Open local `.md` and `.markdown` files
- Save the current document with `Command + S`
- Render headings, links, images, tables, lists, blockquotes, and code blocks
- Syntax-highlighted fenced code blocks in readonly mode
- Code block line numbers and hover language badges in readonly mode
- Render Mermaid fenced code blocks in readonly mode
- Writable editor line numbers
- Writable editor shortcuts:
  - `Command + A` select all
  - `Command + C / V / X` copy, paste, and cut
  - `Command + Z / R` undo and redo
  - `Ctrl + A / E` move to line start and line end
  - `Tab / Shift + Tab` indent and outdent, including multi-line selections
- Drag and drop files into the window
- Open Markdown files from Finder on macOS
- Open external links in the default browser
- macOS app bundle and DMG packaging

## Platform

- macOS on Apple Silicon

## Tech Stack

- Rust
- `tao`
- `wry`
- `pulldown-cmark`
- `syntect`

## Development

Run tests:

```bash
cargo test
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
- `tech-notes/`: technical design notes

## GitHub

<https://github.com/phpple/markhola>
