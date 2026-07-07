# Controller and Flow Test Gaps Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add focused request specs for the controller and flow gaps found in the audit.

**Architecture:** This is primarily test-only work. Add or extend request specs around existing controller behavior; do not change production code unless a new spec reveals a real bug. Keep tests focused on observable HTTP behavior, authorization/scoping, Turbo responses, and persistence.

**Tech Stack:** Rails 8.1, Ruby 3.4, RSpec request specs, fixtures/factories, `task test`.

---

## Tasks

- [x] Cover API v1 member endpoints for locations, medications, schedules, and person medications in `spec/requests/api/v1/resources_spec.rb`.
- [x] Cover `POST /push_subscription/test` success and delivery-failure behavior in `spec/requests/push_subscriptions_spec.rb`.
- [x] Cover `PeopleController#destroy` and `#add_medication` in `spec/requests/people_spec.rb`.
- [x] Add household admin settings request coverage in `spec/requests/admin/settings_spec.rb`.
- [x] Cover notification preference HTML success and failed Turbo update branches in `spec/requests/notification_preferences_turbo_spec.rb`.
- [x] Add direct schedule create/update request specs in `spec/requests/schedules_spec.rb`.
- [ ] Run focused `task test TEST_FILE=...` commands, then `task rubocop` and `task test`.

## Assumptions

- Prefer request specs over component-only tests for user-visible controller flows.
- Keep production changes minimal and only when a new spec exposes an existing behavior gap.
- Preserve unrelated worktree changes, especially the pre-existing `db/schema.rb` modification.
