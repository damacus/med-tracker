# Devin plan review — 5 October 2026

Reviewer: installed Devin CLI, explicit model `swe-2-max`. The user approved
transferring the migration architecture and security details to Devin. A first
read-only source exploration stalled and was cancelled; the completed retry
reviewed supplied repository evidence and the plan, without tools. Exit status 0.

Verdict: conditionally sound. This is a plan review, not approval of source code,
tests, schema compatibility or runtime behavior. Implementation review remains
required for every accepted tranche.

## Material findings and disposition

| Finding | Plan disposition |
| --- | --- |
| Queue reliability must precede enqueueing endpoints | Establish and test Loco PostgreSQL queue early; verify retries, schedules, crash recovery and drain |
| Two schema ledgers need explicit ownership/coexistence | Loco owns subsequent migration; additive changes during rollback; drift/idempotency/schema diff and Rails rehearsal gates |
| Credential/encryption and attachment compatibility unproven | Inventory actual formats and test migrated data before auth implementation acceptance |
| RLS transaction context and workers underspecified | Same transaction/connection context, no silent bypass, negative pooled-connection/worker tests |
| Rails middleware protections can be lost | Add explicit CSRF/CSP/session/cookie/expiry/revocation/rate-limit parity gates |
| SMART/FHIR provider dependency risk | Early maintained-library feasibility review; no custom protocol shortcut |
| Offline capture/replay not explicit enough | Dedicated service-worker and realtime inventory/tests in browser tranche |
| Parity inventory can drift during long migration | Revision-pinned inventory and upstream reconciliation before acceptance |
| Scratch needs timezone support | Embed maintained timezone data or bundle zoneinfo and test scheduling/runtime on both architectures |
| Shared operations should be tested through both entrypoints | Paired browser/API behavior and permission acceptance |
| PR salvage, run modes and dev worker availability need detail | Disposition ledger; dev/test durable worker path; distinct server/worker health and graceful drain |
| Session continuity requires a contract | Inventory and preserve real formats or escalate an unavoidable cutover change before implementation |

## Recommendations not adopted automatically

Devin suggested falling back to another queue library. Official Loco 1.2 docs
document PostgreSQL BackgroundQueue, a visibility-timeout reaper and scheduling;
retain framework support first and evaluate specific missing behavior with tests.
The plan does not assume an unverified dependency is necessary.

Devin attributed offline writes to the Leptos client and speculated about Devise
pepper and Rails signing/encryption formats. These are investigation questions,
not established source findings. Inspect actual owning source before changes.

Theme exports remain directly committed CSS, as the user requested. A bespoke
regeneration/translation tool is not introduced merely on review suggestion.

Sources: <https://loco.rs/docs/how-to/choose-queue-backend/>,
<https://loco.rs/docs/how-to/schedule-recurring-jobs/>.
