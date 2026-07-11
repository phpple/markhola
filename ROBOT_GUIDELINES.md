# Robot Guidelines

## 1. Respect `PLAN.MD`

Do not directly change the plan defined in `PLAN.MD` unless the user explicitly asks for that change.

You may propose suggestions, refinements, and implementation ideas, but they must stay within the direction already defined by `PLAN.MD`.

If something seems useful but has not been explicitly confirmed by the user, do not implement it on your own.

Instead:

- raise it as a suggestion
- explain why it may help
- wait for explicit confirmation before doing it

## 2. Build Only What Is Confirmed

Only implement work that has been clearly requested, confirmed, or can be directly inferred as necessary to complete the confirmed task.

Do not expand scope just because an idea seems better, more complete, or more elegant.

## 3. Suggest Before Expanding Scope

When you identify a potentially valuable improvement that is outside the currently confirmed scope:

1. describe the suggestion briefly
2. explain the benefit
3. wait for the user to confirm before implementing it

## 4. Prefer Incremental Delivery

When implementing new work, prefer the smallest complete change that satisfies the confirmed goal.

Avoid introducing architecture, UI, or product expansion that belongs to a later version unless the user explicitly approves it.

## 5. Keep Technical Plans Aligned With Product Scope

Technical design documents should reflect the current confirmed product scope, not an ideal future product.

Future ideas can be listed as optional follow-up items, but they must be clearly separated from the confirmed implementation scope.

## 6. Use Incremental Git Commits

When code changes are ready, prefer committing them incrementally by feature instead of batching many unrelated updates into one commit.

Commit messages should:

- be written in English
- be a single sentence
- summarize the change clearly and directly

## 7. Follow the Standard Delivery Flow

When implementing a new feature or a user-requested change in this repository, follow this workflow:

1. Confirm the current development version before making changes.
2. Judge the scope of the requested change before assigning the next version:
   - if the work is a bug fix or an improvement to an existing feature, increase the last version number by `+1`
   - if the work is a new feature that did not exist before, increase the middle version number by `+1`
3. If the user directly requests a feature, record or update that feature in `PLAN.MD` under the target version before implementation.
4. Write a technical design document for that feature before coding.
5. After the user accepts the technical design direction, refine the design document and add or update the relevant test cases.
6. For user-visible features, also add or update a matching example Markdown file under `examples/` so users can verify the new capability after installing the new version.
7. Start development only after the user confirms the design, tests, and example direction is OK.
8. Implement the feature in the smallest complete scope that satisfies the confirmed design.
9. Run the existing validation command for the affected code path.
10. Generate the DMG package after the implementation is complete.
11. When the user explicitly says `提交xxx版本`, commit the important milestones incrementally with English one-sentence commit messages.
12. Use the final release commit for that version as the tag target, and create a Git tag named `v版本号`.

For version tracking:

- always keep the current development version aligned across implementation-related files
- do not add feature work without making sure the target version is clear in the plan
- keep `PLAN.MD`, technical design notes, tests, example files, implementation status, and release tag consistent with the same target version
