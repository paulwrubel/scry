## Description

<!-- One or two sentences to describe, at a high level, what this PR does -->

### Related issue

<!-- Link the issue this resolves, e.g. Closes #12. Delete this section if none. -->

Closes #

### Changes

<!-- What does this PR do, and why?

This should NOT be an enumerated list of specific changes with line numbers, we can view the diff for that.

This SHOULD be an unordered list of general changes, a sentence or two for each, and WHY the change was made. -->

## Validation

<!-- Run the full gate before opening this PR. -->

- [ ] `just validate` passes (test --> check --> clippy)
- [ ] If any SQL in a `sqlx::query!` macro or a migration changed: ran `just sqlx-prepare` and committed the regenerated `.sqlx/` cache
- [ ] README CLI reference and `install.sh` updated if commands or flags changed
