# Mobile OAuth product report

## Implemented

- Added mobile OAuth bearer lookup using the Rails URL-safe Base64 SHA-256 token digest. ApiSession lookup and its existing authority path remain first.
- Rejects non-mobile, revoked, expired, incorrectly scoped and stale grants. Login age uses `SESSION_INACTIVITY_TIMEOUT_DAYS` (default 30) and `SESSION_MAX_AGE_DAYS` (default 0, unlimited); malformed values deny OAuth access.
- Rechecks verified account, active linked user, lockout, operational household and current active membership on every request. Medication visibility continues to use that membership under the restricted PostgreSQL role and transaction-local tenant settings.
- Refreshes OAuth `last_used_at` before household selection. Expected household 403/404 and malformed-filter 422 responses commit that authenticated activity; database and audit failures roll back. OAuth reads record `authentication_method: oauth` and `oauth_grant:<id>` in audit context, without bearer material.

## Evidence and state

- The test writer proved red against unchanged Rust code at `7bec286a`: a valid mobile OAuth medication list expected 200 and received 401. Disposable fixture readiness took 122 seconds; the focused HTTP case took 0.01 seconds.
- `task api:test`: 3 passed, 0 failed (initial compile/check 17.0 seconds; after the 422 fix 3.8 seconds).
- `task api:fmt`: passed. `task api:clippy`: passed (initial 10.6 seconds; after the 422 fix 1.6 seconds).
- The runner completed two final isolated Compose acceptance runs, each 19/19 passed and exited 0: 7 OAuth, 9 medication-read and 3 forecast cases. The runner's 43-file pre/post digest matched, but two runner paths were absent from that manifest, so complete byte-for-byte input identity is unproven. Details are in [mobile-oauth-runner-report.md](mobile-oauth-runner-report.md). Independent requirements and code-quality review found no actionable source issue; its evidence limits are in [mobile-oauth-review.md](mobile-oauth-review.md).

Product files remain frozen. The orchestrator owns Git integration and publication.
