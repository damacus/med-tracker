# Medication Take Metrics

Medication take activity is emitted through `ActiveSupport::Notifications` at the `TakeMedicationService` boundary.

## Event Names

| Event | Meaning |
| --- | --- |
| `take_attempted.med_tracker` | A dose-recording request reached the service boundary. |
| `take_recorded.med_tracker` | A medication take was persisted successfully. |
| `take_blocked_by_rules.med_tracker` | Business rules blocked the take before persistence. |
| `take_errors.med_tracker` | The service accepted the request but persistence failed. |
| `dose_taken.med_tracker` | Domain event with medication-take context for in-process subscribers. |

## Labels

Metric events use coarse labels only:

| Label | Description |
| --- | --- |
| `environment` | Rails environment. |
| `role` | Current household membership role when available. |
| `route` | Optional caller-supplied route name. |
| `medicine_context_class` | Source class, such as `Schedule` or `PersonMedication`. |
| `source_type` | Source model name, such as `schedule` or `person_medication`. |
| `error` | Error code for blocked/error events, otherwise blank. |

## Dashboard Panels

Operational dashboards should chart:

| Panel | Source |
| --- | --- |
| Daily take attempts | Count of `take_attempted.med_tracker` grouped by day and `medicine_context_class`. |
| Daily recorded takes | Count of `take_recorded.med_tracker` grouped by day and `medicine_context_class`. |
| Blocked take reasons | Count of `take_blocked_by_rules.med_tracker` grouped by `error`. |
| Take error trend | Count of `take_errors.med_tracker` grouped by day and `error`. |

Alert on any sustained increase in `take_errors.med_tracker` or a sudden spike in
`take_blocked_by_rules.med_tracker` for `cooldown`, `out_of_stock`, or `invalid_source`.
