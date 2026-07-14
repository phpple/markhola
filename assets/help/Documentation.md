# MarkHola Documentation

![Application logo](../logo.png)

Current version: `v0.8.0`

## What is MarkHola

MarkHola is a lightweight desktop Markdown reader and editor for macOS and Windows 11.

## Main Features

- Create a new blank Markdown document
- Open and read local Markdown files
- Edit Markdown content in writable mode
- Switch between readonly and writable modes with `Command + /` on macOS or `Ctrl + /` on Windows
- Save the current file with `Command + S` on macOS or `Ctrl + S` on Windows
- Save the current file to another path with `File > Save As`
- Export the current document to PDF
- Export the current document to HTML
- Print the current document
- Open multiple Markdown files in one window
- Open the built-in documentation from the Help menu
- Use `Command + F` on macOS or `Ctrl + F` on Windows to find text in readonly mode
- Use `Command + F` on macOS or `Ctrl + F` on Windows to find and replace text in writable mode

## Menus

### File

- New
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
- New unsaved documents choose a file path on first save.
- Writable mode supports in-page find, replace, and replace all on the current document.
- Exported and printed output are based on the current rendered document.
- PDF export is available on macOS and Windows 11.
- Printing is available on macOS and Windows 11.
- Windows 11 uses native menus to expose document, edit, tab, and help actions.
