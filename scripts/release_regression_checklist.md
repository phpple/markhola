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
   Expected: launch the app with no document opened, press `Command+W`, and the app exits.

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

6. Multi-document regression
   Open `examples/basic.md` and `examples/multi-document.md`.
   Expected: tabs stay pinned at the top while document content scrolls.
   Expected: switching tabs preserves each document state.
   Expected: closing one of several tabs keeps the app open.
   Expected: closing the last opened document returns to the empty state instead of exiting.

7. Tab menu regression
   Expected: the `Tab` menu can switch tabs, close the current tab, close other tabs, and close all tabs.

8. Theme resource regression
   Expected: `themes/default/layout.css` changes are reflected by the app.
   Expected: packaged app contains `Contents/Resources/themes/default/layout.css`.

9. Inspect regression
   Right click in the preview area.
   Expected: the context menu still exposes `Inspect`.
