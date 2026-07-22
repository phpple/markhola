# Multi-Document Verification

Use this file together with other files under `examples/` to verify multi-document support in MarkHola.

## Suggested Manual Checks

1. Open `examples/basic.md`.
2. Open `examples/languages.md` and confirm a second tab appears instead of replacing the first document.
3. Open `examples/math.md` and confirm a third tab appears.
4. Drag `examples/open-multiple-1.md` and `examples/open-multiple-2.md` into the app together and confirm both files open in separate tabs.
5. Switch between tabs and verify each document keeps its own readonly or writable mode.
6. Edit one tab, leave it unsaved, switch to another tab, and then switch back to confirm the dirty state is preserved.
7. Press `Command + W` to close the current tab and confirm the app stays open while other tabs remain.
8. Close the last remaining tab and confirm the app returns to the empty state.
