# ADR 0020: Telemetry trace identifiers in API errors and the trace header

- Status
  Accepted direction
- Date
  2026-11-02
- Amends
  Refines the API error contract implied by
  [ADR 0007](0007-external-app-integration-contract.md) and the request
  correlation behaviour introduced with `src/request_trace.rs`

## Context

The Rust application assigns every request a random UUID exposed through the
`x-medtracker-trace-id` response header and the completion log field. It is
not a W3C trace identifier, so it cannot join an inbound `traceparent`, cannot
be looked up in a trace backend, and cannot correlate log records in Loki with
spans in VictoriaTraces.

API error bodies currently identify failures only through the stable `code`
and message. When an administrator reports a failed import or a client reports
an API error, there is no identifier that takes an operator straight to the
relevant log and trace records.

## Decision

`x-medtracker-trace-id` now carries the W3C trace ID of the request's server
span: 32 lowercase hexadecimal characters instead of a UUID. When a valid
inbound `traceparent` is present the header returns the caller's trace ID;
otherwise the application-generated trace ID is returned. The header value is
a trace identifier, not a request identifier; the orthogonal `request_id`
correlation is unchanged.

Every JSON API error body gains a `trace_id` field carrying the same 32
lowercase hexadecimal trace ID. Clients and administrators can quote it to
locate the exact request in logs and traces. The field is additive: existing
`code`, `message` and specialised error components are unchanged.

`docs/api/openapi.v1.yaml`, the mirrored
`mobile/android/openapi.v1.yaml`, and the generated clients are updated
together with the change that emits the field.

## Alternatives and consequences

Keeping the UUID and adding a separate W3C header was rejected: two trace-ish
identifiers invite operators to correlate on the wrong one, and the UUID
could never join inbound context or a trace backend. Minting a
`traceparent`-style combined ID was rejected for the same reason; the bare
trace ID is the value both Loki records and the trace store index.

Clients that parsed `x-medtracker-trace-id` as a UUID must accept the 32-hex
form; both are opaque to well-behaved callers. Error-body consumers see one
new optional-key field, which additive-change tolerant clients ignore.
