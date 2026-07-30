# ADR 0004: Domain Events with ActiveSupport::Notifications

- Status: Accepted
- Date: 2026-04-14

## Context

MedTracker records audit history with PaperTrail, but it does not currently publish domain events for important medication operations. That forces any secondary behaviour such as notifications or analytics to infer intent by polling database state instead of reacting to the operation that just happened.

Issue `#1051` identifies four initial domain events:

- `DoseTaken`
- `MedicationRestocked`
- `ScheduleCreated`
- `LowStockThresholdReached`

The minimum tranche needed now is to choose an event mechanism and implement `DoseTaken` plus `LowStockThresholdReached`.

## Decision

We will use `ActiveSupport::Notifications` as the in-process domain event mechanism.

Event names for the initial tranche:

- `dose_taken.med_tracker`
- `low_stock_threshold_reached.med_tracker`
- `take_attempted.med_tracker`
- `take_recorded.med_tracker`
- `take_blocked_by_rules.med_tracker`
- `take_errors.med_tracker`

Publishers will emit raw notification payloads directly at the domain/service boundary. We are not introducing a custom event bus, wrapper object, or subscriber framework in this tranche.

## Rationale

### Why `ActiveSupport::Notifications`

1. It is already part of Rails and needs no new dependency.
2. It is lightweight and sufficient for in-process publication.
3. It keeps the first event tranche small and easy to review.
4. It gives us a standard subscription API for later notification and analytics consumers.

### Why not a custom event bus

A custom bus would add structure before we have even proven the event set, naming, or subscriber needs. That would increase design surface area without solving a concrete problem in this tranche.

### Why not `wisper` or another gem

An additional gem would add dependency and pattern overhead for a problem Rails already solves adequately for best-effort in-process notifications.

## Consequences

### Positive

- Domain intent is published at the point where it occurs.
- Notifications and analytics can subscribe without coupling themselves to controllers or models.
- The implementation stays small and aligned with current Rails conventions.

### Negative

- These are best-effort in-process events, not durable integration events.
- Subscribers are not replayable if the process crashes after the database commit.
- We still need follow-up work for additional publishers and subscribers.

## Follow-up

Deferred from this ADR tranche:

- `MedicationRestocked`
- `ScheduleCreated`
- notification subscribers
- analytics subscribers
- any durable or cross-process event transport

## Follow-up: Operational signals

The application observability work completed after this decision adds a
privacy-safe operational-event boundary beside domain publication.
`ActiveSupport::Notifications remains the in-process domain-event mechanism`.
The operational boundary does not replace, wrap, or become a subscriber framework.
It records safe attempts and outcomes independently so that a
logging failure cannot alter domain behaviour and subscriber ordering cannot
hide the publication attempt.

Domain payloads may remain rich enough for trusted in-process subscribers.
Operational records use event-specific allowlists and must not serialize those
raw payloads. A helper can emit the safe operational record before invoking
`ActiveSupport::Notifications`, and can record a subscriber failure before
re-raising it, but the original synchronous propagation semantics remain
unchanged.

This ADR remains accepted. It should be superseded only if a later decision
replaces or wraps the domain-event mechanism itself.
