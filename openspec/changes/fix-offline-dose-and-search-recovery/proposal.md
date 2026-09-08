## Why

Offline dose recording can offer unavailable doses, select exhausted stock, and abandon retries after temporary failures. Search failures look like empty results. Tracking issue: https://github.com/damacus/med-tracker/issues/2127.

## What Changes

- Preserve queued doses on transient, transport and authentication failures; atomically retain rejected doses and expose recovery status.
- Select stock using all pending consumption and prevent overlapping local queue submissions.
- Show cached server eligibility and prevent unavailable or stale dose actions, including pending-dose conflicts.
- Distinguish search failures from empty results and support retry.
- Add failure-path regression tests in four stacked PRs.

Non-goals: Sorbet expansion, native clients, changing medication safety rules, automatic administration, replacing the UI framework or modifying immutable dose history.

## Capabilities

### New Capabilities

- `offline-dose-recovery`: Durable browser queue, stock selection and conservative cached eligibility.
- `search-failure-recovery`: Accessible search failure and retry states.

### Modified Capabilities

None. The existing medication-take-sync specification describes the batch API, not the browser offline endpoint.

## Impact

Offline snapshot generation, IndexedDB queue, offline Stimulus controls, global search, and focused request/browser tests. The server remains authoritative for permissions, timing and inventory. No database migration or new dependency is planned.
