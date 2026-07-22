# MarkHola Documentation

![Application logo](../logo.png)

Current version: `v0.7.7`

## What is MarkHola

MarkHola is a lightweight desktop Markdown reader and editor for macOS.

## Main Features

- Create a new blank Markdown document
- Open and read local Markdown files
- Edit Markdown content in writable mode
- Switch between readonly and writable modes with `Command + /`
- Save the current file with `Command + S`
- Save the current file to another path with `File > Save As`
- Export the current document to PDF
- Export the current document to HTML
- Print the current document
- Open multiple Markdown files in one window
- Switch app shell themes from the View menu
- Toggle fullscreen viewing from the View menu
- Open the built-in documentation from the Help menu
- Use `Command + F` to find text in readonly mode
- Use `Command + F` to find and replace text in writable mode

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

### View

- Theme > Default
- Theme > GitHub
- Theme > Dark
- Theme > Light
- Toggle Full Screen

### Help

- Documentation

## Notes

- Readonly mode renders Mermaid, math, tables, links, images, and code blocks.
- Readonly mode supports in-page find across the current rendered document.
- Writable mode keeps the Markdown source editable.
- New unsaved documents choose a file path on first save.
- Writable mode supports in-page find, replace, and replace all on the current document.
- Exported and printed output are based on the current rendered document.
