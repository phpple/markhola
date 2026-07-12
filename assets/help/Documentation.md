# MarkHola Documentation

Current version: `v0.8.0`

## What is MarkHola

MarkHola is a lightweight desktop Markdown reader and editor for macOS and Windows 11.

## Main Features

- Open and read local Markdown files
- Edit Markdown content in writable mode
- Switch between readonly and writable modes with `Command + /` on macOS or `Ctrl + /` on Windows
- Save the current file with `Command + S` on macOS or `Ctrl + S` on Windows
- Save the current file to another path with `File > Save As` on macOS or the in-window action bar on Windows
- Export the current document to PDF on macOS
- Export the current document to HTML
- Print the current document on macOS
- Open multiple Markdown files in one window
- Open the built-in documentation from the Help menu on macOS or the in-window action bar on Windows
- Use `Command + F` on macOS or `Ctrl + F` on Windows to find text in readonly mode
- Use `Command + F` on macOS or `Ctrl + F` on Windows to find and replace text in writable mode

## Menus

### File

- Open
- Save
- Save As
- Print
- Export > PDF
- Export > HTML
- Close
- Exit

### Edit

- Undo / Redo
- Cut / Copy / Paste
- Select All
- Find

### Tab

- Next Tab
- Previous Tab
- Close Tab
- Close Other Tabs
- Close All Tabs

### Help

- Documentation

## Notes

- Readonly mode renders Mermaid, math, tables, links, images, and code blocks.
- Readonly mode supports in-page find across the current rendered document.
- Writable mode keeps the Markdown source editable.
- Writable mode supports in-page find, replace, and replace all on the current document.
- Exported and printed output are based on the current rendered document.
- PDF export and printing are currently available on macOS.
- Windows 11 exposes Open, Save, Save As, Export, Documentation, and About through the in-window action bar.
