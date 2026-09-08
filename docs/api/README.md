# API contract conventions

The OpenAPI v1 contract is defined in [openapi.v1.yaml](openapi.v1.yaml).

The [portable data format](portable-data.md) defines record fields used by
mobile snapshots, sync, export, and import.

The [API versioning policy](versioning.md) defines compatible changes,
deprecation, stable generated names, and generated-client ownership.

## Canonical addressing

The document's first server URL is `/api/v1`. Path keys are relative to that
server and must begin with `/` without repeating `/api/v1`. The
`/households/{household_id}/people` path resolves to
`/api/v1/households/{household_id}/people`.

## Operation IDs

Every HTTP operation must declare a unique, stable lower-camel-case
`operationId`. Use an action-aware verb: `list` for collection reads, `get`
for singular reads, `create`, `update`, `replace`, or `delete` for CRUD, and a
precise verb such as `pause`, `resume`, `adjust`, `test`, or `dryRun` for a
custom action. PATCH and PUT operations on the same resource need distinct
IDs.

## Tags

Every operation has exactly two tags: one audience tag and one resource tag.
The audience tag is one of `Public`, `Account`, `Household`, or `Household
administration`; the resource tag identifies the narrowest responsible API
domain. Define each used tag once in the top-level `tags` array with a concise
description, and do not define tags that no operation uses.

## Medication pause periods

Discover `medication_pause_periods.supported` in `/api/v1/capabilities` before using this feature. The capability lists the five public reasons. If it is absent, clients must report that the server does not support recording pause context.

- `GET /api/v1/households/:household_id/medication_pause_periods` lists visible current and completed periods newest first with normal `page` and `per_page` pagination. Supply `source_type` and portable `source_id` together for one source's history. Authorised history remains visible after retirement; retired sources cannot be paused or resumed.
- `POST` to the same path accepts `{ "medication_pause_period": { "source_type": "schedule", "source_id": "portable UUID", "reason": "out_of_supply", "note": "Delivery tomorrow" } }`. The other source type is `person_medication`. The reason is required; the note is optional.
- `POST /api/v1/households/:household_id/medication_pause_periods/:portable_id/resume` accepts no attributes. It closes only the addressed period. A retry for an older completed period cannot close a newer pause.

Writes require manage access to the source person. Start and end times are server acceptance times. Clients cannot backdate or edit periods. Both writes support `Idempotency-Key`; resume also supports `If-Match`. Responses contain the portable period and source IDs, retained reason and note, start/end timestamps, actor membership IDs and display names, and the legacy-context flag. Unknown legacy start times and actors remain null.

Schedule and direct-assignment reads add `current_pause_period` when pause context is loaded. It is null when there is no open pause. Historical reasons and notes are available through the history endpoint.

The existing schedule and direct-assignment pause/resume operations are deprecated in favour of these operations. They remain compatible for at least two minor releases and 90 days after the first released replacement, as required by [the versioning policy](versioning.md). Old pause requests still accept an empty body and record `reason_not_recorded`. This is an unreleased deprecation notice; release notes must name the first released replacement and earliest removal date before the notice period begins.
