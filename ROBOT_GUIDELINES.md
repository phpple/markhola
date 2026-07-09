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
