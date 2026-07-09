# MarkHola

MarkHola is a lightweight desktop Markdown reader built with Rust for Apple Silicon Macs.

## Features

- Readonly Markdown preview
- Open local `.md` and `.markdown` files
- Render headings, links, images, tables, lists, blockquotes, and code blocks
- Drag and drop files into the window
- Open external links in the default browser
- macOS app bundle and DMG packaging

## Platform

- macOS on Apple Silicon

## Tech Stack

- Rust
- `tao`
- `wry`
- `pulldown-cmark`

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

## GitHub

<https://github.com/phpple/markhola>
