# Code Review - codex/global-search-command-palette

**Base Branch**: origin/main
**Changed Files**: 21
**Changed Ruby Files**: 14
**Review Date**: 2026-05-06
**Review Skills**: review-ruby-code, sandi-metz-reviewer, rubycritic availability check, simplecov availability check

---

## Summary

This branch adds a keyboard-first global search command palette backed by a Pundit-scoped Rails service, authenticated `/search` endpoint, PostgreSQL trigram indexes, authenticated chrome triggers, Stimulus keyboard behavior, and request/service/system coverage.

The implementation is generally sound: model search goes through policy scopes, wildcard search terms are escaped with `sanitize_sql_like`, query-like params are filtered from logs, and the system specs cover the high-risk keyboard path. I did not find a critical security or authorization issue in the reviewed changes.

## Critical Issues

No critical issues found.

## Design & Architecture

### OOP / Sandi Metz Findings

Warning: [GlobalSearchQuery is 228 lines and owns search orchestration, five record-specific query builders, result shaping, command matching, scoring, sorting, normalization, and route generation](file:///Users/damacus/.codex/worktrees/8e6c/med-tracker/app/services/global_search_query.rb#L3). This exceeds the Sandi Metz 100-line class rule and is much larger than most existing query objects in this app. The current version is acceptable as an MVP, but the next search expansion should extract per-type searchers or a small registry before adding more result types.

Warning: [GlobalSearchQuery#call returns blank-query command results directly](file:///Users/damacus/.codex/worktrees/8e6c/med-tracker/app/services/global_search_query.rb#L37), bypassing the service-level `limit` contract applied by `sort_results`. Today the default command set is small, so the UI is not broken, but `GlobalSearchQuery.new(..., query: '', limit: 2).call` can return more than two results. Add a blank-query limit spec and apply `.first(limit)` or route blank commands through the same sorter.

### Rails Patterns

No N+1 issue found in the added record searches. [Medication results preload location](file:///Users/damacus/.codex/worktrees/8e6c/med-tracker/app/services/global_search_query.rb#L67), and [schedule/person-medication results join and include their display associations](file:///Users/damacus/.codex/worktrees/8e6c/med-tracker/app/services/global_search_query.rb#L97).

The service-object shape matches the repo's existing `*Query` naming, but the branch introduces a larger orchestration query than the surrounding patterns. Existing query objects are mostly narrow, so future work should prefer small collaborators such as `GlobalSearch::PeopleSearch` and `GlobalSearch::MedicationsSearch` over continuing to grow this class.

## Security Concerns

No new security issue found.

Positive observations:

- [All record searches route through `Pundit.policy_scope!`](file:///Users/damacus/.codex/worktrees/8e6c/med-tracker/app/services/global_search_query.rb#L196).
- [Search terms are escaped with `ActiveRecord::Base.sanitize_sql_like`](file:///Users/damacus/.codex/worktrees/8e6c/med-tracker/app/services/global_search_query.rb#L225).
- [The request spec covers unauthenticated JSON access](file:///Users/damacus/.codex/worktrees/8e6c/med-tracker/spec/requests/global_search_spec.rb#L10), scoped user results, admin visibility, carer visibility, and wildcard escaping.
- [Filtered parameters now include `q`, `query`, and `search`](file:///Users/damacus/.codex/worktrees/8e6c/med-tracker/config/initializers/filter_parameter_logging.rb#L8).

## Test Coverage

Focused search tests passed after resetting the stale test stack:

- `env COVERAGE=true task test TEST_FILE='spec/services/global_search_query_spec.rb spec/requests/global_search_spec.rb spec/system/global_search_spec.rb'`
- 16 examples, 0 failures

Full suite passed after rebuilding the test environment:

- `task test:rebuild`
- `task test`
- 2103 examples, 0 failures, 2 pending

RuboCop passed:

- `task rubocop`
- 900 files inspected, no offenses detected

Coverage gap:

- [The limit spec covers a nonblank query](file:///Users/damacus/.codex/worktrees/8e6c/med-tracker/spec/services/global_search_query_spec.rb#L57), but not the blank-query command-shortcut path where the current service bypasses `sort_results`.

## Tool Reports

### RubyCritic Summary

RubyCritic metrics are unavailable in this worktree:

- `rubycritic --version` failed because `rubycritic` is not installed on PATH.
- The local RubyCritic helper, `scripts/check_quality.sh`, auto-installs RubyCritic and may modify the Gemfile, so I did not run it during a review-only pass.

### SimpleCov Summary

SimpleCov metrics are unavailable:

- Running focused specs with `COVERAGE=true` produced no `coverage/` directory.
- No SimpleCov configuration or gem entry was found in the app paths checked during this review.

## Recommendations

1. Fix the blank-query `limit` path in `GlobalSearchQuery#call` and add a service spec for `query: ''`.
2. Before expanding v2 search, split `GlobalSearchQuery` into small per-type query objects or a result-source registry. That keeps the MVP class from becoming the place every search concern accumulates.
3. If RubyCritic/SimpleCov reports are required for reviews, add repo-native `task` wrappers that do not auto-edit the Gemfile during analysis.

## Positive Observations

- The branch keeps v1 internal search simple: Rails, PostgreSQL, Pundit, and Stimulus only.
- The keyboard behavior is covered end to end, including Ctrl+K/Cmd+K, focus, arrow navigation, Enter navigation, Escape, and the mobile trigger.
- The SQL uses static fragments plus bind parameters; I did not find user-controlled SQL interpolation.
- The implementation avoids searching medication takes, audit logs, notes, and user records in v1.
