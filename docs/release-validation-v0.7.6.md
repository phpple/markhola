# Release Validation Record: v0.7.6

- Date: 2026-07-21
- Candidate DMG: `dist/MarkHola-0.7.6.dmg`
- Candidate app bundle: `dist/MarkHola.app`
- Validation app copy: `/tmp/markhola-0.7.6-validation/MarkHola.app`
- Validated runtime target during final checks: `/Users/xiaolongdeng/Documents/markhola/dist/MarkHola.app/Contents/MacOS/MarkHola`

## Key lesson captured by this validation

Local validation cannot assume the foreground `MarkHola` window belongs to the newest build.

This run initially failed because macOS routed requests to `/Applications/MarkHola.app`, which shared the same bundle id as the release candidate.

Final validation only became reliable after:

1. identifying the wrong running app copy
2. stopping the conflicting `/Applications/MarkHola.app`
3. launching the candidate app directly from the release artifact path
4. checking process-path evidence before trusting UI behavior

## Automated checks completed

- `cargo test`
  - Result: passed (`49 passed, 0 failed, 1 ignored`)
- `./scripts/release_regression.sh --with-package`
  - Result: passed for build/packaging/resource checks
  - Note: PDF/print smoke checks in this sandbox were downgraded to warnings for known WKWebView JavaScript limitations
- `./scripts/package_dmg.sh`
  - Result: produced `dist/MarkHola-0.7.6.dmg`

## Manual / sandbox checks completed

- Opened Markdown file successfully from the packaged app
- Switched document to writable mode
- Edited Markdown content
- Saved content to disk
- Switched back to readonly mode
- Verified rendered output updates after save
- Verified `[toc]` rendering updates after save
- Verified added heading appears in TOC
- Verified additional markdown markers render correctly:
  - strikethrough
  - bold
  - italic
  - inline code
- Verified fullscreen can be toggled without losing the current document

## Theme / View menu note

The `View` / `Theme` functionality is implemented in the candidate artifact, but this environment exposed an accessibility inconsistency:

- runtime/process evidence confirmed the candidate app was the correct `0.7.6` build
- the release artifact contained the theme resources
- the same environment did not reliably expose the `View` menu through the automation accessibility tree during final packaged-app validation

This mismatch is why the release-validation rules now require process-path and startup-log evidence, not UI observation alone.

## Release decision guidance

- Do not publish unless the final GitHub upload uses this exact DMG candidate
- If a future local validation disagrees with runtime logs, first rule out bundle-id collisions and wrong-app activation before judging the candidate artifact
