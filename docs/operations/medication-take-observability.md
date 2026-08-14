# Medication Take Observability

MedTracker writes privacy-safe medication workflow events as JSON records on
standard output. These events help operators distinguish a blocked request from
a database failure or rollback. MedTracker does not provide medication-specific
Prometheus metrics or dashboards.

`MedicationAdministration::RecordDose` is the normal dose-recording boundary.
Portable record restoration and historical data migration do not represent new
dose activity, so they do not emit this workflow.

## Event sequence

| Event | Outcome | Meaning |
| --- | --- | --- |
| `medication_take.attempted` | `unknown` | A dose-recording request reached the service. |
| `medication_take.persisted` | `unknown` | The row was written inside the current transaction. This result is provisional. |
| `medication_take.committed` | `success` | The outer transaction committed. This is the successful outcome. |
| `medication_take.rolled_back` | `failure` | The outer transaction rolled back after provisional persistence. |
| `medication_take.blocked` | `failure` | A business rule stopped the request before persistence. |
| `medication_take.failed` | `failure` | Persistence, household loading, or an unexpected operation failed. |

A successful request emits `attempted`, `persisted`, and `committed`. A rollback
replaces `committed` with `rolled_back`. A request blocked before persistence
emits `attempted` and `blocked`.

## Safe fields

Medication workflow records can include these bounded fields:

| Field | Meaning |
| --- | --- |
| `medtracker.source.category` | `schedule` or `person_medication`. |
| `medtracker.actor.role` | The current household role, when available. |
| `medtracker.reason` | A stable code such as `requested`, `committed`, `out_of_stock`, or `persistence_failed`. |
| `medtracker.workflow.id` | Opaque identifier shared by one workflow. |
| `medtracker.attempt.id` | Opaque identifier for one attempt. |

The records exclude household, person, medication, schedule, and take
identifiers. They also exclude medication names, dose values, timestamps,
request parameters, and exception messages. See
[Application observability](application-observability.md) for the complete
privacy and schema contract.

## Querying logs

Search the application log dataset for `event.name`. Use the workflow and
attempt identifiers to correlate records. Do not infer success from
`medication_take.persisted`; require `medication_take.committed` for a completed
take.

Useful operational checks include:

- compare attempted and committed event counts over the same period;
- group blocked events by `medtracker.reason`;
- investigate failed and rolled-back events;
- check that each committed event has a related attempt.

Any metrics or dashboards must be derived in the downstream observability
system from these bounded events. MedTracker does not provide those
aggregations.

## Source contracts

The event mappings are defined in `lib/observability/event_mapper.rb` and
`lib/observability/medication_transaction_outcome.rb`. The complete signal
inventory is `config/observability/signal_registry.yml`.
