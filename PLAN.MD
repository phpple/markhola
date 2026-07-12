# MarkHola

This is another markdown reader and exporter software.Written by AI, free are the significal features for it.


## Basis Features

### v0.5.0
1. Readonly
   Open markdown file and can show the basic elements:
   - title, h1,h2,h3...
   - link
   - picture
   - table
2. Language: english only
3. Platform: Apple M4

### v0.6.0
1. Writable
   - You can edit the markdown file under this model.After you enter "Command + /", you can switch the model both for readonly and writeonly.
   - You can save the markdown file after edit it and enter the "Command + S"

### v0.6.1
1. Code Highlight Support
  - Display line number at the left side
  - Highlight the keyword by the type of the language
  - Display the language on the right top corner while the mouse hover the code area
2. Writable editor shortcut enhancement
  - Support "Command + A" to select all text while in writable mode
  - Support "Command + C", "Command + V", and "Command + X" for common text editing
  - Support "Command + Z" to undo the previous editor action
  - Support "Command + R" to redo the next editor action
  - Support "Ctrl + A" to move caret to the start of the current line
  - Support "Ctrl + E" to move caret to the end of the current line
  - Support "Tab" to insert 4 spaces and indent selected multiple lines
  - Support "Shift + Tab" to remove 4 spaces and outdent selected multiple lines
3. Writable editor line numbers
  - Display line numbers at the left side while editing Markdown

### v0.6.2
1. Mermaid diagram support
   - Render Mermaid fenced code blocks in readonly mode
   - Support common Mermaid diagram types such as flowchart, sequence, class, state, ER, gantt, pie, journey, git graph, mindmap, timeline, and sankey
   - Keep Mermaid source text editable in writable mode

### v0.6.3
1. Mainstream language syntax highlight enhancement
   - Improve readonly fenced code block highlighting coverage for mainstream languages
   - Ensure the languages already listed in `examples/languages.md` render with syntax highlighting when a language token is declared
   - Add language alias normalization for common fenced code block tokens such as `typescript`, `swift`, and `kotlin`
   - Keep unknown or untagged code blocks rendering safely as plain text fallback
2. Mathematical expression support
   - Support GitHub-style inline math expressions in readonly mode
   - Support GitHub-style block math expressions delimited by `$$...$$`
   - Support fenced `math` code blocks as display math in readonly mode
   - Keep raw math source editable in writable mode

### v0.7.0
1. Multiple Markdown documents in one app window
2. Theme directory for app shell styling

### v0.7.1
1. Support for Exporting PDF

### v0.7.2
1. Support for printing Markdown files
2. Save As for Markdown documents
3. Export Markdown documents to HTML
4. Built-in documentation entry in the Help menu
5. Bug fixes
   - Reload the current document correctly after using the page reload action

### v0.7.3
1. In-page find support in readonly mode
2. In-page find and replace support in writable mode
3. Code split. Reduce the single rs file size, split to three parts: interface,implements,tests.

### v0.7.4
1. Support `[toc]`
2. Enable to choose multiple markdown files while use "open file"

### v1.0.0
1. Two models: Readonly、writeonly.
2. Readonly:Default mode, you can open any markdown file.
   Support features:
   - Table
   - Link
   - Picture
   - Flow
   - Math Expression
   - Code Highlight
3. Writeonly:You can edit the markdown file under this model.After you enter "Command + /", you can switch the model both for readonly and writeonly.
4. Export to PDF. 
5. Support: English, Chinese
6. Platform: Apple Intel、Apple M、Windows 11

## Tech architecture
1. Written by Rust.
2. Embbed a chrominum browser for display markdown, support H5 labels.
