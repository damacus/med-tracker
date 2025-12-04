---
trigger: model_decision
description: When writing tests, modifying spec files, or adding new features
---

# Testing Strategy

## Framework & Organization

- **Framework**: RSpec for all tests (`_spec.rb`)
- **System Tests**: Capybara for end-to-end user interaction tests
- **Test Data**: Standard Rails fixtures in `spec/fixtures/`
- **API Mocking**: VCR cassettes in `spec/vcr_cassettes/`
- **Organization**: Follow Rails/RSpec conventions (`spec/models`, `spec/features`, `spec/policies`, etc.)

## TDD Requirements

- Write failing test BEFORE implementation code
- Follow Red-Green-Refactor cycle strictly
- Test public APIs only, not implementation details

## Coverage Requirements

- Every authorization path ([show?](cci:1://file:///Users/damacus/repos/damacus/med-tracker.worktrees/end-to-end/app/policies/person_medicine_policy.rb:7:2-9:5), [create?](cci:1://file:///Users/damacus/repos/damacus/med-tracker.worktrees/end-to-end/app/policies/person_medicine_policy.rb:11:2-13:5), [update?](cci:1://file:///Users/damacus/repos/damacus/med-tracker.worktrees/end-to-end/app/policies/person_medicine_policy.rb:19:2-21:5), [destroy?](cci:1://file:///Users/damacus/repos/damacus/med-tracker.worktrees/end-to-end/app/policies/person_medicine_policy.rb:27:2-29:5)) must have explicit test coverage
- Policy methods require tests for: admin, clinician, self, carer, parent, and unauthorized users
- When adding conditional checks like [self_or_dependent?](cci:1://file:///Users/damacus/repos/damacus/med-tracker.worktrees/end-to-end/app/policies/person_medicine_policy.rb:37:2-45:5), test ALL branches
- New model validations require both positive and negative test cases
- Admin CRUD flows need tests for: success path, validation errors, duplicate handling, and immediate usability

## Fixture Guidelines

- Fixtures must represent realistic data
- Validate all relationships in fixtures
- Ensure no duplicate fixtures with same unique attributes

## Capybara Guidelines

- Use exact element text/labels from the actual views (check view files, not assumptions)
- Prefer `click_link` for `<a>` tags, `click_button` for `<button>` tags
- Use `fill_in` with the actual field label or `name` attribute
