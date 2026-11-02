## Purpose

Defines a private, durable, retention-bounded trace path that lets operators
query and correlate MedTracker OpenTelemetry traces in the existing Grafana
observability stack, now backed by single-node VictoriaTraces and enabled for
the canary deployment only.

## MODIFIED Requirements

### Requirement: Accepted traces are durable for a bounded retention window

The VictoriaTraces backend SHALL store accepted trace data on a dedicated
persistent volume, SHALL retain it for 7 days, and SHALL delete expired data
without sharing credentials or write authority with unrelated workloads.

#### Scenario: Backend restarts after accepting a trace

- **GIVEN** an accepted trace has been flushed to durable storage
- **WHEN** the trace-backend workload restarts
- **THEN** the trace remains queryable during its retention window

#### Scenario: Trace reaches the retention boundary

- **GIVEN** a stored trace is older than 7 days
- **WHEN** retention processing completes
- **THEN** the expired trace is no longer queryable
- **AND** storage is reclaimed according to the backend retention contract

#### Scenario: Unrelated workload requests storage credentials

- **GIVEN** a workload is not the trace backend or its storage provisioner
- **WHEN** it attempts to read the trace-storage credential
- **THEN** the credential is unavailable to that workload

#### Scenario: Durable storage becomes unavailable

- **GIVEN** the trace backend cannot write or read its durable store
- **WHEN** it handles ingestion or query traffic
- **THEN** the failure is visible through health, metrics, or alerts
- **AND** the backend does not report failed persistence as durable success

### Requirement: Operators can query and correlate traces in Grafana

Grafana SHALL provision a healthy Tempo-type data source named
`victoriatraces` (VictoriaTraces `/select/tempo` API) that supports trace-ID
lookup, bounded TraceQL search, trace-to-log navigation, and log-to-trace
navigation through the existing Loki data source via a derived `trace_id`
field.

#### Scenario: Operator searches by trace identifier

- **GIVEN** a retained trace identifier
- **WHEN** an operator queries it in Grafana
- **THEN** Grafana displays the trace and its parent-child span structure

#### Scenario: Operator searches MedTracker traces

- **GIVEN** retained traces contain safe service and deployment resource
  attributes
- **WHEN** an operator runs a bounded TraceQL search for MedTracker in a
  selected time range
- **THEN** matching traces are returned without searching unrelated services

#### Scenario: Operator navigates from a trace to logs

- **GIVEN** a displayed span has a trace identifier and matching Loki records
  exist
- **WHEN** the operator follows the trace-to-logs link
- **THEN** Grafana opens a time-bounded Loki query scoped by the trace and
  MedTracker service

#### Scenario: Operator navigates from a log to a trace

- **GIVEN** a Loki record contains a valid retained `trace_id`
- **WHEN** the operator follows the derived-field trace link
- **THEN** Grafana opens the matching trace in the `victoriatraces` data
  source

#### Scenario: Correlation metadata contains sensitive data

- **GIVEN** a trace or log record would expose medication, person, household,
  credential, token, cookie, request-body, or raw domain-identifier data
- **WHEN** the signal reaches Grafana
- **THEN** the prohibited value is absent
- **AND** correlation uses opaque identifiers and conforming attributes

### Requirement: Trace export is promoted through an exact-revision isolated verification

MedTracker trace export SHALL be enabled for the `medtracker-canary`
deployment only and SHALL remain disabled for production `home` and every
other workload until the exact candidate image and configuration pass finite
ingestion, query, correlation, privacy, and fail-open acceptance checks in
canary.

#### Scenario: Backend is ready before isolated verification

- **GIVEN** the trace backend and Grafana data source are healthy
- **WHEN** the rollout begins
- **THEN** only the medtracker-canary deployment is configured to export
  traces
- **AND** production `home` remains configured with trace export disabled

#### Scenario: Isolated verification emits a safe acceptance trace

- **GIVEN** medtracker-canary runs the exact candidate image configured for
  OTLP/HTTP export
- **WHEN** synthetic request, error and import paths execute
- **THEN** their traces are queryable in Grafana within 15 minutes
- **AND** the traces contain the expected safe service, deployment, trace,
  span, and correlation fields
- **AND** matching Loki records link to the same trace identifiers

#### Scenario: Trace export fails during a medication operation

- **GIVEN** the backend is unavailable or rejects an export
- **WHEN** MedTracker handles a medication or health-data operation
- **THEN** the operation is not blocked, rolled back, or altered by the
  telemetry failure
- **AND** the failure is observable without sensitive payload data

#### Scenario: Isolated acceptance fails

- **GIVEN** any required canary acceptance check fails
- **WHEN** promotion is evaluated
- **THEN** no further deployment enables trace export
- **AND** rollback restores the last known-safe canary export configuration

#### Scenario: Isolated acceptance passes

- **GIVEN** the finite canary acceptance matrix passes for an immutable image
  digest
- **WHEN** trace export is later proposed for another deployment
- **THEN** it uses the same tested endpoint, protocol, privacy, and
  failure-isolation contract
- **AND** the deployed digest is recorded with the acceptance evidence
### Requirement: The trace backend is operationally bounded and verifiable

The VictoriaTraces deployment SHALL be single-node with pinned dependencies,
explicit resources, health checks, scrapeable backend metrics, a network
policy allowing only medtracker-canary pods to reach the OTLP/HTTP endpoint,
focused repository validation, and a reversible rollout that does not depend
on local Compose services.

#### Scenario: GitOps configuration is rendered

- **GIVEN** the trace backend, storage, Grafana, and MedTracker configuration
  changes
- **WHEN** focused repository validation runs
- **THEN** it verifies chart pinning, rendered resources, internal service
  ports, credentials wiring, retention, network policy, data-source
  configuration, and staged exporter settings

#### Scenario: Backend becomes unhealthy

- **GIVEN** the trace backend is unavailable, repeatedly restarting, rejecting
  spans, or failing durable storage operations
- **WHEN** cluster monitoring evaluates its health
- **THEN** operators receive a bounded diagnostic signal
- **AND** the signal contains no trace payload data

#### Scenario: Backend resources grow unexpectedly

- **GIVEN** trace ingestion or query load exceeds the expected low-volume
  envelope
- **WHEN** resource or storage limits are approached
- **THEN** the condition is observable before unbounded cluster impact
- **AND** remediation can disable MedTracker export without changing
  application behavior

#### Scenario: Local development starts

- **GIVEN** a developer starts the existing MedTracker Compose environment
- **WHEN** the trace-backend change has been applied to the cluster
- **THEN** no additional local VictoriaTraces, collector, or Grafana service
  is required by default
