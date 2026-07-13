# Release Regression Checklist

Run automated regression first:

```bash
./scripts/release_regression.sh
```

Run this extended command when packaging a release candidate:

```bash
./scripts/release_regression.sh --with-package
```

## Manual checks

1. Empty launch close behavior
   Expected: launch the app with no document opened, press the primary close shortcut (`Command+W` on macOS or `Ctrl+W` on Windows), and the app exits.

2. Basic Markdown rendering
   Open `examples/basic.md`.
   Expected: headings, links, blockquotes, tables, and images render normally.

3. Code highlight regression
   Open `examples/languages.md`.
   Expected: fenced blocks show line numbers and language badges.
   Expected: `typescript`, `swift`, and `kotlin` blocks are highlighted instead of plain fallback.

4. Mermaid regression
   Open `examples/mermaid.md`.
   Expected: Mermaid blocks render diagrams rather than remaining as plain code.

5. Math regression
   Open `examples/math.md`.
   Expected: inline math, `$$...$$`, and fenced `math` blocks render correctly.

6. PDF export regression
   Open `examples/pdf-export.md`.
   Expected: `File > Export > PDF` exports the current active tab only.
   Expected: the exported PDF keeps headings, table, code block, image, and math content.
   Expected: exporting from writable mode includes unsaved edits.

7. HTML export regression
   Open `examples/basic.md`.
   Expected: `File > Export > HTML` exports the current active tab as a standalone HTML file.
   Expected: the exported HTML keeps rendered Markdown styling and can load Mermaid and math enhancements.

8. Print regression
   Open `examples/basic.md` and `examples/mermaid.md`.
   Expected: `File > Print` and the platform print shortcut both open the system print panel for the current active tab.
   Expected: the print panel content reflects the current document instead of the application shell.
   Expected: writable-mode unsaved edits are included in the printed content.

9. Find regression
   Open `examples/basic.md`.
   Expected: the primary find shortcut (`Command + F` on macOS or `Ctrl + F` on Windows) and `Edit > Find` on macOS open the same find panel.
   Expected: readonly mode highlights matches, shows a stable match count, and supports `Enter`, `Shift + Enter`, `Next`, and `Previous`.
   Expected: writable mode can find, replace, and replace all within the current tab without breaking dirty state updates.

10. Documentation regression
   Expected: `Help > Documentation` opens the bundled release help markdown file inside the app.

11. Multi-document regression
   Open `examples/basic.md` and `examples/multi-document.md`.
   Expected: tabs stay pinned at the top while document content scrolls.
   Expected: switching tabs preserves each document state.
   Expected: closing one of several tabs keeps the app open.
   Expected: closing the last opened document returns to the empty state instead of exiting.

12. Tab menu regression
   Expected on macOS: the `Tab` menu can switch tabs, close the current tab, close other tabs, and close all tabs.

13. Theme resource regression
   Expected: `themes/default/layout.css` changes are reflected by the app.
   Expected: packaged app contains `Contents/Resources/themes/default/layout.css`.
   Expected: packaged app contains `Contents/Resources/help/Documentation.md`.

14. Inspect regression
   Right click in the preview area.
   Expected: the context menu still exposes `Inspect`.
