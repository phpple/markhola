# Release Publish Workflow

Use this workflow for every MarkHola release candidate that will be uploaded to GitHub.

The key rule is:

1. finish implementation and version updates
2. build the exact DMG candidate
3. validate that candidate inside a macOS sandbox
4. only publish the GitHub release after sandbox validation passes

## 1. Prepare the release candidate

Make sure the target version is already aligned in:

- `Cargo.toml`
- `PLAN.MD`
- `README.md`
- `assets/help/Documentation.md`
- any release notes or user-facing example files affected by the release

Run the automated regression flow first:

```bash
./scripts/release_regression.sh --with-package
```

This should leave you with a packaged release candidate at:

```bash
dist/MarkHola-<version>.dmg
```

## 2. Run pre-publish sandbox validation

Before creating or publishing the GitHub release, validate the packaged app in a macOS sandbox environment.

The validation target should be the exact DMG file that will be uploaded to GitHub, not a separately rebuilt artifact.

Recommended sandbox validation flow:

1. Mount `dist/MarkHola-<version>.dmg`
2. Copy `MarkHola.app` from the mounted volume into a sandbox-local path
3. Before launching, check whether other local `MarkHola.app` copies already exist, especially `/Applications/MarkHola.app`
4. Stop or isolate other running `MarkHola` processes so LaunchServices does not route validation to an older installed copy
5. Launch that copied app
6. Confirm the running process path matches the copied candidate app, not `/Applications/MarkHola.app` or another local bundle
7. Capture startup-log evidence that the expected version and release-specific initialization ran in that candidate app
8. Verify the app can open a Markdown file through `File > Open`
9. Switch to writable mode
10. Edit the document and add representative Markdown syntax
11. Save the file and confirm the file changed on disk
12. Switch back to readonly mode and verify rendered output
13. If the release includes `[toc]`, verify the generated table of contents updates after save

Hard rule:

- Do not trust UI validation alone when multiple MarkHola installs share the same bundle id.
- Always keep at least one process-path or startup-log proof that the tested app is the copied candidate artifact from the target DMG.

Minimum required manual coverage:

- open a Markdown file successfully
- edit and save successfully
- verify the release's new feature works
- verify one or more existing core features still work

For `v0.7.5`-style releases, the sandbox verification must include:

- `[toc]` rendering
- multi-section heading navigation in the generated TOC
- normal Markdown editing and saving

If sandbox validation fails, do not upload or publish the DMG.

If the UI behavior and the logs disagree, assume the wrong app copy may have been activated first, then re-run validation against a confirmed candidate process path.

## 3. Create the GitHub release draft

Only after the sandbox checks pass:

1. create the Git tag `v<version>` on the final release commit
2. draft the GitHub release
3. upload the already-validated DMG file
4. fill the release title and notes

The release notes should summarize the items listed under the matching version in `PLAN.MD`.

## 4. Publish the GitHub release

Publish the release only after confirming all of the following:

- the uploaded DMG is the same validated artifact
- the release title matches `MarkHola-<version>`
- the release notes match the target version scope
- the Git tag points at the intended final release commit

## 5. Keep evidence

For each release, keep a short verification record with:

- the DMG path
- the copied validation app path
- the running process path used during validation
- the tested version
- the sandbox validation result
- the key behaviors verified
- the GitHub release URL after publish
