---
trigger: model_decision
description: When completing a feature, finishing a task, or before committing code
---

# Definition of Done

Before considering work complete:

## Code Quality

- [ ] `bundle exec rubocop` returns no errors
- [ ] No new RuboCop disables added without justification

## Testing

- [ ] `task test` passes with no failures
- [ ] New code has corresponding test coverage
- [ ] No tests were deleted or weakened

## Documentation

- [ ] Comments match implementation (or removed if misleading)
- [ ] Complex logic is self-documenting via method names

## Review Readiness

- [ ] Commit messages follow Conventional Commits format
- [ ] Changes are atomic and focused
- [ ] Created a GitHub PR with clear description and linked issues
