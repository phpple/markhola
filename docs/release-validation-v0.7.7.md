# Release Validation Record: v0.7.7

- Date: 2026-07-25
- Candidate DMG: `dist/MarkHola-0.7.7.dmg`
- Candidate DMG SHA-256: `e3a312c2f8c99c5e444deab6064d0234d562564096eaac19382c0b1bb4ccab59`
- Mounted candidate: `/Volumes/MarkHola 9/MarkHola.app`
- Validation app copy: `/tmp/markhola-0.7.7-release-validation.pVMhe5/MarkHola.app`
- Validated process: PID `51320`
- Validated process path: `/tmp/markhola-0.7.7-release-validation.pVMhe5/MarkHola.app/Contents/MacOS/MarkHola`
- Candidate executable SHA-256: `0531a6097267354d67c5a625fd3b8a9108f220597c9da88cf11710473033b496`

## Artifact Evidence

- The mounted and copied candidate executables had identical SHA-256 values.
- `CFBundleShortVersionString` was `0.7.7`.
- `codesign --verify --deep --strict` passed.
- Startup logs reported `version=0.7.7 platform=macos/aarch64`.
- Packaged help, logo, and all four theme resources were present.

## Automated Validation

- `./scripts/release_regression.sh --with-package` passed.
- Unit tests: `52 passed, 0 failed, 1 ignored`.
- PDF export smoke test passed.
- Mermaid PDF export smoke test passed.
- HTML export smoke test passed.
- Basic and Mermaid print preparation smoke tests passed.
- Mermaid print page-count smoke test passed with 6 pages.
- Multi-file path extraction keeps all file URLs in order and ignores non-file URLs.

## Candidate App Validation

- Empty workspace rendered with the native footer pinned at the bottom.
- Native footer displayed the empty path and ready status.
- Persisted dark theme loaded on candidate startup and matched the native footer.
- Opened a copied Markdown file through `File > Open`.
- Switched to writable mode, edited the file, and saved it.
- Confirmed the saved content changed on disk.
- Switched back to readonly mode and confirmed the new heading and paragraph rendered.
- Confirmed footer path, word count, line count, mode, and status updated.

## Drag-and-Drop Note

The final Finder cross-window drag gesture was not completed through UI automation. The release keeps the native Wry drag-drop callback implementation, dispatches every dropped path through the existing `OpenPath` flow, and retains automated multi-path extraction and workspace tab coverage.

