# Verification and publication

The sweep uses local test and development environments with synthetic fixtures. Canary is not
confirmation evidence: it needs the latest release before a separate live check.

Current source and locked dependencies run in the cached Ruby 4.0.6 Docker runtime. A fresh image
build could not retrieve base-image metadata. The isolated test project uses PostgreSQL 18 and
Playwright 1.61 with Chromium revision 1234; it does not use the shared host test database.

The focused red/green results and exact screenshot provenance are recorded in the numbered
reports. Those counts overlap and must not be added together as a full-suite result.

## Final gates

- Locale trees: passed, five files in sync.
- Full Rails suite: passed, **5,585 examples, zero failures, one pending**, in 9 minutes 33 seconds.
  The pending OmniAuth auto-link example requires OIDC configuration. The first combined run
  exposed six stale assertions; the four corrected test files then passed 34 focused examples
  before the successful complete rerun. No product code changed between those full runs.
- Ruby lint: passed, 1,836 files inspected with no offences; the final four assertion files also
  passed their focused lint check.
- Documentation build: passed (`task docs:build`, no issues). Whitespace checked before commit.
- Independent review and coordinator image inspection: accepted, including the corrected
  enlarged profile labels after their font-size transition completes.

## Follow-ups

- [#2124](https://github.com/damacus/med-tracker/issues/2124): the task runner can skip a supplied
  command when an internal task is invoked twice.
- [#2125](https://github.com/damacus/med-tracker/issues/2125): as-needed availability appears in
  routine dashboard totals. This needs a separate presentation-contract decision.

The coordinator accepted the final source, test corrections and screenshot evidence. Publication
uses branch `codex/fix-web-ui-sweep` against `main`. The work belongs to
[#2123](https://github.com/damacus/med-tracker/issues/2123).
