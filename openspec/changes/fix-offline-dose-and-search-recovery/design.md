## Context

See proposal.md for motivation. Browser IndexedDB holds tenant-scoped queued and failed doses. The offline snapshot already uses policy scopes, and RecordDose validates all synced administrations. The offline UI currently duplicates only part of the eligibility and inventory rules.

## Goals / Non-Goals

Goals: retain user-entered history across failures, make pending status understandable, and prevent misleading dose actions.

Non-goals: reproduce all Rails dosing algorithms in JavaScript, grant authority from a cache, or relax sync validation.

## Decisions

1. Stack order: sync durability/recovery; pending stock selection; cached dose eligibility; search failure/retry. Each layer includes regression tests and updates this checklist.
2. Retain transient failures (408, 429, 5xx, network and malformed responses) in queued storage. Stop the pass on transient/auth failures. Move permanent rejections to failed storage in one transaction covering both stores. Keep UUIDs for all retries. Expose retry and sign-in-needed status without response bodies or sensitive diagnostics. Reject the alternative of deleting before writing failure state.
3. Read pending takes when selecting stock, and synchronously guard repeated queue clicks. Use the same pending list for rendering and selection. Do not silently discard queued administrations.
4. Add offline-only eligibility metadata using existing Rails source and overlapping-dose rules and record policies. Include the effective dose and evaluation date. Disable actions when metadata is unavailable/stale or a pending dose for the same person and medicine makes eligibility uncertain. This conservative rule avoids an incomplete JavaScript reimplementation of taper, cycle and overlap logic. Sync always reauthorizes using fresh server state; cached metadata is only a UI hint.
5. Search checks HTTP status and response shape, renders a translated accessible error with retry, and retains the query. Abort remains silent. Empty results require a successful valid response.

## Risks / Trade-offs

- Cached eligibility can become stale or be revoked → restrict it to the evaluated date, clearly label cached/pending state, and revalidate every sync on the server.
- Conservative pending-dose blocking limits repeated offline doses for one medicine → explain that sync is needed before another dose; never claim full offline clinical validation.
- Server eligibility checks can add snapshot queries → keep them at the controller/service boundary and test realistic sources; no queries in components.
- Storage failure → preserve the original queued record through atomic rollback and show a recoverable status.
- Browser failures differ from request failures → exercise real IndexedDB and fetch failure paths in browser specs.

## Migration Plan

No schema upgrade is needed. Older cached snapshots without eligibility metadata remain readable but dose actions require refreshing. Deploy the stack in order. Reverting UI changes preserves existing stored queues and UUIDs.
