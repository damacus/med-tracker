## Context

See proposal.md for the six requested workflows. `/api/v1` already supplies household-bound authentication, person-scoped policies, idempotency keys, ETags, snapshots, sync batches, and generated contracts. Stock removals exist on main. Dose occurrences do not yet have storage or a resolver; their two existing OpenSpec changes remain authoritative for domain semantics. ADRs 0006, 0009 and 0010 require concrete source references, immutable takes, shared domain owners, and delivery adapters.

## Goals / Non-Goals

**Goals:** Add native delivery without duplicating domain rules; make retries and revoked access safe; finish one layer and its tests before starting the next.

**Non-Goals:** Generic arbitrary-model mutation, native screen redesign, relaxed MFA, mutable takes, and automatic merge or deployment.

## Decisions

### One ordered GitHub stack

Start with this planning PR against refreshed main. Follow with dose storage, projection/resolution, consumers, API/contracts and routine-source support as the existing outcome plans require. Then deliver stock removals, locations, reviews/reports, profile/invitations, and offline parity in that order. Split each area further when a migration, shared service, or generated contract merits independent review. Every PR targets the immediately lower branch. Use `gh stack link` to register and extend the remote stack, retaining all existing members. Do not put all six features in one implementation PR.

### Reuse the planned occurrence domain

Implement the unmet prerequisites in `record-scheduled-dose-outcomes` and `extend-dose-outcomes-to-routine-assignments`, not a second not-taken table. Storage is additive, household-constrained, uniquely identified per source/window/position, portable, and audited. Projection excludes PRN, respects pause intervals, and performs no writes. A source lock serializes take/not-taken races. The resolver calls `MedicationAdministration::RecordDose` for administrations. Corrections reopen only not-taken outcomes; a persisted take is never edited or deleted. Consumers must stop reminders for resolved occurrences before advertising writable support.

### Thin stock and location adapters

Stock-removal endpoints call `RemoveMedicationStockService`, preserving its quantity, source, reason, note, audit and duplicate-submission rules. Add bounded history and decimal-string schemas. Do not model a removal as an absolute correction. Location writes use existing policies and explicit household assignment. Membership writes authorize both the location action and selected person's management. Reuse retained-history deletion guards; do not invent retirement state in this stack. Use portable locators consistently and require ETags for mutable record updates/deletes.

### Shared review and report queries

Extract a shared review filter/update boundary only where web logic currently owns reusable behaviour. Preserve practitioner fields and evidence snapshots. Reports reuse `Reports::*`, existing date bounds, person-selection policies and PDF rendering. Supply typed JSON for native rendering and protected PDF responses for export; render failures return a stable API error without clinical data. No public attachment URL or long-lived permission cache is introduced.

### Identity stays online

Profile updates permit the current web profile's non-security fields only: date of birth, time zone, Gravatar preference and mobile shortcuts. Avatar upload uses a separate authenticated multipart operation with current image type/size validation and protected reads; removal acts on the current person's attachment. Account email/password/roles remain outside profile writes. Accept invitations under an authenticated identity with verified matching email, pending token, expiry/revocation checks, and the existing acceptance transaction. Session/household selection must still issue authority through normal auth flows. Resend requires the existing fresh privileged-action check and invalidates the old token as the web does. Never queue access changes offline.

### Explicit offline operation matrix

Publish support for dose create/link, outcome not-taken/reopen, stock removal, medication create/update/inventory/order/receive, dosage-option create/update, health-event create/update/delete, person create/update, location create/update/guarded-delete, and schedule/direct-assignment create/update/retire/pause/resume/reorder. Review updates are supported with ETags. Profile, avatars, invitation/membership/access administration and report exports remain online-only, with stable unsupported errors. Location-person membership changes also remain online because they affect visibility.

Each offline operation calls the same owning service and rechecks current access at replay time. Preserve all-or-nothing batch semantics, deterministic result order, client UUID/idempotency protection, ETag conflict responses and rollback of audit/sync effects. No arbitrary method dispatch or unrestricted attribute hash is allowed. Retained history prevents destructive deletion. Capabilities enumerate support rather than asserting blanket offline parity.

### Contracts travel with behaviour

Each endpoint has a stable operation ID, typed request/response/error schemas, string IDs and decimal quantities, household/person authorization tests, replay/conflict cases, and bounded collections where applicable. Keep root OpenAPI authoritative. Update checked-in generated root clients; adopt pins in native build roots only where present and verified. Do not create a second iOS import. Run repository API generation/determinism tasks and relevant client checks.

## Risks / Trade-offs

- Outcome prerequisite scope is larger than an endpoint change → Follow existing bounded tranches and keep unresolved tasks visible.
- Concurrent replay can duplicate clinical or stock events → Source locks, database constraints, shared idempotency and failed-transaction retries at the outer boundary.
- Location and invitation changes can widen access → Re-authorize current actor and target person, enforce tenant foreign keys, and test revoked/stale credentials.
- Report generation or uploads can fail after partial work → Bounded inputs, transactional metadata changes, protected attachment handling and PHI-safe errors.
- Native pins can lag → Publish explicit capability negotiation and update generated artifacts alongside their backend layer.

## Migration Plan

Add and verify outcome storage first without exposing writes. Complete each domain dependency and consumer before enabling API capability flags. Subsequent routes and schemas are additive. Retain occurrence/audit rows on rollback; never reverse by dropping clinical history. Publish each validated layer to the stack and hand off for separate merge/deployment review.

### Report contract details

JSON report reads and `.pdf` downloads have separate operation IDs so native
generators retain correct response types. Health history uses the existing
GP query and its date bound. Medicine-review reports remain current queue
snapshots with the web status filter; date filters are rejected rather than
silently applied to a different review date. Both require a selected person
and record successful downloads before sending the response.
