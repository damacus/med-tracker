# Tasks

## 1. Establish failing privacy coverage

- [x] 1.1 Safely integrate current main while preserving proposal and unrelated work; record base commit and inspect all AI outbound call sites and existing settings operations.

  Implementation base: `859575cbd` (fast-forward from `763b8718e`). Outbound sites:
  candidate discovery and synthesis in `ai_suggestions::call_provider`, plus
  trusted-source fetching in `fetch_source`. Settings use the shared household
  administration operation for both browser and API updates.
- [x] 1.2 Add full-App acceptance tests using a counting local provider for absent/false consent, opted-in success, withdrawal, foreign household and revoked membership; run focused Task tests and observe the expected failure before implementation.

  Red evidence: `rtk task test:integration TARGET=care_api FILTER=ai_consent_defaults_off`
  returned 200 with a provider-derived draft where 404 was required. The expanded
  `FILTER=ai_consent` batch also exposed the missing controller settings allowlist
  (400 on explicit consent). No outbound enforcement was present for these runs.

## 2. Persist and manage consent

- [x] 2.1 Add forward migration, fresh schema/entity support and default-off state; verify existing/new households and relevant import paths cannot acquire implicit consent using focused persistence tests.
- [x] 2.2 Extend existing settings operations, API contracts and audited manager-only updates; first observe failing tests, then verify unauthorised actors, CSRF, rollback, omitted fields and repeated submissions with focused Task tests.
- [x] 2.3 Add the settings control and translated external-provider disclosure; verify enabled/disabled persistence, keyboard/label/error behaviour and desktop/mobile screenshots under docs/screenshots using the browser. Update relevant user documentation with this change.

## 3. Enforce consent at outbound boundaries

- [x] 3.1 Add consent to shared authorisation and coordinate every outbound suggestion send with consent changes; make zero-request and positive-control acceptance tests pass without bypassing existing authority, plan or feature checks.
- [x] 3.2 Add and first observe a failing deterministic withdrawal race test; verify that after withdrawal commits neither new suggestions nor later provider calls send data. Verify unavailable consent storage fails closed and errors/logs contain no clinical payload.
- [x] 3.3 Document concurrency semantics and update OpenAPI/client representations where changed; verify contract checks and focused AI tests, keeping only explicit AI-positive fixtures opted in.

## 4. Integrated verification and review

- [x] 4.1 Run required `rtk task ci`, relevant integration/browser Task entries, `rtk task docs:build` and `rtk git diff --check`; record actual outcomes and any infrastructure blocker without claiming unrun checks passed.
- [x] 4.2 Review the final diff against all spec scenarios, record the test evidence and implementation status, and update task checkboxes only for completed work. Keep production untouched and leave publication to the delivery controller.

## Verification evidence

- `rtk task test:integration TARGET=care_api CLI_ARGS='-E "test(ai_suggestions) | test(household_settings) | test(portable_v3_import_restores_labels)"'`: 15 passed. Includes all six counting-provider privacy cases, existing AI access/plan/input/evidence cases, settings rollback/replay and successful import retaining false consent.
- `rtk task test:integration TARGET=requests FILTER=household_settings`: 13 passed against the OpenAPI contract, including credential scopes, stale/revoked credentials, permission ordering, replay and rollback.
- `rtk task test:integration TARGET=persistence FILTER=household_ai_consent`: 1 passed; existing rows survive the forward migration with false consent and new households remain false.
- `rtk task test:browser FILTER=household_ai_consent`: 1 passed; explicit opt-in, persisted withdrawal, CSRF refusal, validation state, keyboard operation and desktop/mobile screenshots verified. Images are in `docs/screenshots/household-ai-consent/`.
- `rtk task docs:build`: passed. Locale-tree checker passed across all five supported languages. Strict OpenSpec validation and `git diff --check` passed.
- API and client generation: `rtk task mobile:android:api:update` passed, including adapter tests and generation integrity checks.
- Full CI initially found the synthetic household YAML missing the new required entity field. Added explicit false; the existing fixture consistency test then passed. Full final CI output is retained in `/private/tmp/medtracker-ai-ci-final.log`.
- Final `rtk proxy task ci` passed (the proxy preserves complete logs): 305 Rust unit tests, both canary checks, all 1,226 integration tests, the competing-process fixture test and all 74 selected browser journeys passed. Formatting, strict Clippy, dependency checks, JavaScript tests and doctests also passed. Incidental tracked screenshots generated by the broad suite were restored; the two new AI consent screenshots were retained.

The withdrawal race was first observed failing with an HTTP 200 provider-derived
draft after withdrawal committed. With per-send locking, the same test returns
404, observes only the already-started source fetch and no later provider call,
and rejects the next request without increasing the outbound count.

Review checked both provider call sites and source fetching, the shared manager
authority and audit transaction, strict boolean parsing, omitted-field handling,
default-off seed/migration/import behaviour and PHI-safe error conversion. No new
dependency, raw SQL exception or live-data action was introduced. Publication
remains with the delivery controller.
