# MarkHola

![MarkHola logo](assets/logo.png)

MarkHola is a lightweight desktop Markdown reader and editor built with Rust for Apple Silicon Macs.

## Current Version

- `v0.7.6`

## Features

- Readonly and writable modes with `Command + /` mode switching
- Create a new blank Markdown document with `File > New` or `Command + N`
- Open local `.md` and `.markdown` files (Open File supports multi-select)
- Open and keep multiple Markdown documents in one window
- Export the current document to PDF from `File > Export > PDF`
- Export the current document to HTML from `File > Export > HTML`
- Print the current document from `File > Print`
- Switch app shell themes from `View > Theme > Default / GitHub / Dark / Light`
- Load the app shell themes from editable files under `themes/<theme>/layout.css`
- Save the current document with `Command + S`
- Save a new unsaved document by choosing a path on first save
- Save the current document to another path with `File > Save As`
- Open the bundled documentation from `Help > Documentation`
- Render headings, links, images, tables, lists, blockquotes, and code blocks
- Syntax-highlighted fenced code blocks in readonly mode
- Improved mainstream language highlight coverage for fenced code blocks
- Mathematical expressions in readonly mode, including inline math, `$$...$$`, and fenced `math` blocks
- Code block line numbers and hover language badges in readonly mode
- Render Mermaid fenced code blocks in readonly mode
- Support `[toc]` placeholder for table of contents in readonly mode
- In-page find in readonly mode with `Command + F`
- In-page find and replace in writable mode with `Command + F`
- Writable editor line numbers
- Writable editor shortcuts:
  - `Command + A` select all
  - `Command + C / V / X` copy, paste, and cut
  - `Command + Z / R` undo and redo
  - `Ctrl + A / E` move to line start and line end
  - `Tab / Shift + Tab` indent and outdent, including multi-line selections
- `Command + W` close the current document tab
- Drag and drop files into the window
- Toggle fullscreen document viewing from `View > Toggle Full Screen`
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

Use the full release publish workflow before uploading a GitHub release:

```bash
open scripts/release_publish_workflow.md
```

Build the app:

```bash
cargo build
```

Create the macOS app bundle and DMG:

```bash
./scripts/package_dmg.sh
```

Release order for GitHub publishing:

1. `./scripts/release_regression.sh --with-package`
2. sandbox-validate the exact DMG candidate
3. create the GitHub release draft and upload that validated DMG
4. publish the release only after the sandbox checks pass

## Project Structure

- `src/`: desktop app source code
- `src/bin/make_icns.rs`: macOS icon generation helper
- `assets/`: logo and icon sources
- `examples/`: sample Markdown files for manual verification
- `scripts/`: packaging scripts
- `scripts/release_publish_workflow.md`: pre-publish sandbox validation and GitHub release workflow
- `themes/`: directly editable app theme files
- `assets/help/`: bundled in-app help markdown files

## GitHub

<https://github.com/huimang/markhola>
