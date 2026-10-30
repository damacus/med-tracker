# ADR 0019: Service unavailable for persistence and backing-service failures

- Status
  Accepted direction; implementation verified by integration contract tests
- Date
  2026-10-29
- Amends
  Refines the API error taxonomy implied by
  [ADR 0007](0007-external-app-integration-contract.md) without changing any
  resource or authorisation contract

## Context

Every `DbErr` raised inside MedTracker request handling is normalised to
`OperationError::Unavailable` in `src/models/errors.rs`. Until the
person-owned-schedules delivery, the three JSON API error mappers — the
household care mapper (`src/controllers/api/care/response.rs`), the
account-session mapper (`src/controllers/api/account_sessions.rs`) and the
household sync mapper (`src/controllers/api/care/sync.rs`) — materialised that
variant as HTTP 500 with the `internal_error` code.

HTTP 500 asserts an unexpected server fault. A database or backing service that
cannot answer is neither unexpected nor a defect in the request: it is a
transient unavailability that callers are expected to retry. Reporting it as
`internal_error` trains clients to treat operator-side outages as
non-retryable defects and hides the distinction between "MedTracker is broken"
and "MedTracker cannot currently reach its store".

The production mapper change landed in the fix commit
`fix(care): resolve adoption 500s and map unavailable errors to 503`; this ADR
records the deliberate contract decision and its verification sweep.

## Decision

Materialise `OperationError::Unavailable` — and the equivalent
`AuthenticationError::Unavailable` — as HTTP 503 `Service Unavailable` with
the `service_unavailable` code and the message "Service is temporarily
unavailable" on every path served by the three mappers:

- `/api/v1/households/**` responses produced by `care/response.rs`;
- `/api/v1/auth/**` responses produced by `account_sessions.rs`, including
  `/auth/guardian-declarations` and `/auth/sessions` routes;
- `/api/v1/households/{id}/sync/**` responses produced by `sync.rs`.

Request-audit `security_audit_events` metadata records the materialised
status, so audit rows for these failures record `503`.

Genuine server faults remain HTTP 500 `internal_error`. In particular:

- Better Auth's own `/api/auth/*` routes materialise `AuthError::internal`
  as 500 independently of these mappers;
- the `account_sessions.rs` catch-all still emits 500 for non-taxonomy
  failures;
- endpoint-specific unavailability keeps its specialised code and component
  (`avatar_unavailable`, `report_unavailable`, `GuardianDeclarationUnavailable`,
  `CapabilitiesUnavailable`, `InvitationDeliveryUnavailable`) where the
  contract already distinguishes it.

`docs/api/openapi.v1.yaml` documents `503` for every exercised operation whose
failure path can reach a changed mapper, via a generic
`components.responses.ServiceUnavailable` or the existing specialised
component. `500` stays documented only where a real 500 emitter remains
reachable on that path.

## Alternatives and consequences

Retaining 500 for persistence failures was rejected: it conflates retryable
dependency outages with programming defects and contradicts the retry
guidance given to sync and native clients. Introducing per-endpoint bespoke
codes for generic unavailability was also rejected; specialised components
are reserved for failures the caller can meaningfully distinguish.

Clients that previously special-cased `internal_error`/500 from these paths
must accept `service_unavailable`/503. The change is observable, so the
sweep updated every exercised status, code and audit-metadata assertion and
the OpenAPI document in the same commit as the contract decision it
verifies.
