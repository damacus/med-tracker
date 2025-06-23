---
trigger: always_on
---

# Testing Strategy

1. Framework: Use RSpec for all tests (`_spec.rb`).
2. System Tests: Use Capybara for end-to-end user interaction tests.
3. Test Data: Use standard Rails fixtures for test data. Ensure fixtures are well-organized and represent realistic data.
4. API Mocking: Use VCR to record and replay HTTP interactions with external services. Store cassettes in `spec/vcr_cassettes`.
5. Organization: Follow Rails/RSpec conventions for test file locations (`spec/models`, `spec/features`, etc.).
