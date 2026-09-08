## Why

[Issue #2152](https://github.com/damacus/med-tracker/issues/2152) tracks six gaps that make native clients depend on the web for routine care, inventory, setup, and review workflows. Complete these through the existing household API and shared domain services, with additive contracts and sequential, reviewable GitHub PRs.

## What Changes

- Deliver scheduled and direct routine dose occurrence reads, not-taken outcomes, audited reopen/replacement, and compatible take linkage. Reuse `record-scheduled-dose-outcomes` and `extend-dose-outcomes-to-routine-assignments`; implement their required domain foundations first.
- Add attributable stock-removal creation and history using the existing inventory service.
- Add location creation, editing, guarded deletion, and person location-membership management.
- Expose the medicine-review queue, practitioner review updates, health-history reports, and medicine-review reports.
- Add current-profile updates, protected avatar upload/removal, and authenticated invitation acceptance/resend.
- Extend offline sync writes to the new care and inventory operations plus supported online record mutations, with an explicit operation matrix. Keep security-sensitive account/access operations online.
- Publish typed OpenAPI schemas, capability discovery, contract tests, and generated client updates with each applicable layer.

## Capabilities

### New Capabilities

- `mobile-dose-outcome-api`: Native access to the existing planned outcome semantics.
- `mobile-stock-removal-api`: Record and read stock loss without recording administration.
- `mobile-location-management-api`: Manage locations and their person memberships.
- `mobile-review-report-api`: Read and update reviews and retrieve protected reports.
- `mobile-profile-invitation-api`: Manage the current profile and complete household invitation workflows.
- `mobile-offline-write-parity`: Explicit, replay-safe offline support for care and inventory mutations.

### Modified Capabilities

None. Existing dose-outcome changes own their domain requirements; this change coordinates their implementation and adds delivery requirements without replacing them.

## Impact

Rails models/services/policies where prerequisite domain behaviour is missing; API controllers, sync, serializers, routes, protected attachments, report services, OpenAPI, request/domain tests, and generated clients. Existing scheduled-outcome plans also cover consumers, reminders, and portable-v2 preservation. No new external service or dependency is planned.

## Non-goals

New native screen design, direct edits/deletion of persisted medication takes, guessed historical outcome backfills, offline changes to identity or access, replacement of hosted OIDC/MFA, new root-record retirement policy, deployment, and merging the stack. An Undo before submission remains a client concern; this work does not make persisted takes mutable.
