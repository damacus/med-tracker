# Weekly handoff: 26 September 2026

## Resume here

Continue on `codex/openapi-contract-20260925` in
`/tmp/medtracker-openapi-contract-20260925`. The objective remains completing
the original 89 API operations. Keep the immutable baseline in
`remaining-89-baseline.json`; route presence alone does not count as completion.
Use `coverage/test-evidence.json` and `coverage/report.md` for current gaps.

This session ends after the required dosage-field remediation and publication.
Do not start another feature as part of weekly closure. Next week, finish the
remaining explicit dosage/auth assertions before taking the bounded people
work described in `people-followup.md`.

## Decisions to preserve

- Six required dosage fields must be NOT NULL in PostgreSQL and non-optional
  in Rust. Rails model validation alone did not enforce database nullability.
- Invalid API writes remain 422. There is no special legacy-record API path.
- No invalid production data has been observed. The migration must stop if
  existing required values are missing, without inventing medical values or
  deleting rows. Production migration and deployment have not been performed.
- Ordinary household APIs accept app tokens with household binding and active
  credential checks. SMART integration grants remain outside that authority.
- Use the persistent sole-writer charter, bounded work and isolated Compose
  networks. Do not reproduce known Rails defects as parity requirements.

## Preserved work outside this branch

The earlier journey/UI checkout at
`/Users/damacus/.codex/worktrees/56c9/med-tracker`, branch
`codex/axum-leptos-port-plan`, contains paused, uncommitted API/UI changes,
screenshots and reports. It was inspected read-only during closure. Do not
clean, reset or merge it implicitly. This branch's publication does not publish
or verify that paused work.

## Verification and publication

The schema report records exact commands and logs. Post-migration verification
passed Rails 6,104 examples with zero failures, the focused migration tests
2/2, dosage HTTP 13/13, Rust units 15/15, Rust check and clippy. Shared app-token
session regression passed 8/8 before the schema-only change. Independent
requirements and code review passed in `dosage-schema-review.md`.

Remaining work is tracked in
[issue #2299](https://github.com/damacus/med-tracker/issues/2299).
API completion, UI/PWA parity and the application-process memory target remain
unfinished; no cutover readiness is claimed.
