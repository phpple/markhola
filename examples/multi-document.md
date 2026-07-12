# Multi-Document Verification

Use this file together with other files under `examples/` to verify multi-document support in MarkHola.

## Suggested Manual Checks

1. Open `examples/basic.md`.
2. Open `examples/languages.md` and confirm a second tab appears instead of replacing the first document.
3. Open `examples/math.md` and confirm a third tab appears.
4. Switch between tabs and verify each document keeps its own readonly or writable mode.
5. Edit one tab, leave it unsaved, switch to another tab, and then switch back to confirm the dirty state is preserved.
6. Press `Command + W` on macOS or `Ctrl + W` on Windows to close the current tab and confirm the app stays open while other tabs remain.
7. Close the last remaining tab and confirm the app returns to the empty state.
