---
name: markhola-release-artifact-validation
description: Use when validating a MarkHola release candidate DMG before GitHub publish, especially when multiple local MarkHola installs may exist and the validation must prove it is running the exact packaged artifact rather than an older app copy.
---

# MarkHola Release Artifact Validation

Use this skill during the final pre-publish verification of a MarkHola DMG.

The main failure mode this skill prevents is validating the wrong app copy, especially when:

- `/Applications/MarkHola.app` already exists
- multiple `MarkHola.app` bundles share the same bundle id
- `open -a` or LaunchServices routes document-open or activation events to an older installed app

## Required rules

1. Validate the exact DMG artifact that will be uploaded, not a separately rebuilt app.
2. Treat bundle-id collisions as normal. Never assume the foreground MarkHola window belongs to the candidate artifact.
3. Collect process-path or startup-log evidence before trusting UI observations.

## Required validation flow

1. Build the candidate DMG.
2. Mount the DMG.
3. Copy `MarkHola.app` from the mounted volume into a validation-local path.
4. Before UI validation, inspect or clean up other running `MarkHola` processes, especially `/Applications/MarkHola.app`.
5. Launch the candidate app directly from its copied bundle path or executable path.
6. Capture evidence that the running process path matches the copied candidate bundle.
7. Capture app startup logs and confirm the expected version and menu/feature initialization appear there.
8. Only then begin UI validation for open/edit/save/preview/release-feature checks.

## Mandatory evidence

Before declaring the release candidate valid, keep at least:

- the candidate DMG path
- the copied validation app path
- the running process path
- one startup log excerpt proving the expected version/features initialized

## If the UI and logs disagree

Prefer logs and process-path evidence first.

Typical examples:

- Logs show the new `View` menu is installed, but the visible app still looks old.
- A document-open action appears to do nothing, but the app that received the event was not the candidate artifact.

When this happens:

1. assume validation is targeting the wrong app copy until proven otherwise
2. stop relying on `open -a` alone
3. relaunch the candidate artifact directly
4. re-check running process paths
5. repeat UI validation only after the runtime target is confirmed

## Publish gate

Do not upload or publish the GitHub release until:

- the validated app is proven to come from the candidate DMG
- the tested process path matches that copied artifact
- the release feature passes on that confirmed runtime target
