# API contract conventions

The OpenAPI v1 contract is defined in [openapi.v1.yaml](openapi.v1.yaml).

The [portable data format](portable-data.md) defines record fields used by
mobile snapshots, sync, export, and import.

The [API versioning policy](versioning.md) defines compatible changes,
deprecation, stable generated names, and generated-client ownership.

## Current profile

`GET` and `PATCH /households/{household_id}/profile` read and update the
membership person linked to the signed-in account. The existing `/me` response
retains its identity contract. Profile identifiers are strings.

Submit a `profile` object with `date_of_birth`, `time_zone`, `gravatar_enabled`
or `mobile_shortcuts`. Person and account changes save in one transaction.
Unknown fields, including email and roles, are rejected. These operations are
online only. Current person permissions are checked before cached retries.

`PUT /households/{household_id}/profile/avatar` accepts a multipart `avatar`
file: PNG, JPEG or WebP, up to 5 MiB. `GET` on that path streams the image after
checking current person access; `DELETE` removes the attachment. No public or
presigned URL is returned. Invalid uploads and storage failures retain the
existing image. Avatar requests run online and do not cache multipart responses
under `Idempotency-Key`.

## Invitation acceptance

`POST /invitations/accept` accepts a `token` for the verified account behind a
user API session. Household app tokens and delegated OAuth grants cannot use
this account-level operation. The invitation email must match the account.

Acceptance creates the target-household person, membership and intended grants
in one transaction using the web signup grant workflow. Expired, revoked or
mismatched invitations and existing membership conflicts return the same
`invitation_unavailable` error. A retry returns the current active membership
without recreating revoked grants. The API does not issue a new credential;
use the existing household-selection login flow for the new household.

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
## Stock removal

Inventory managers can create and list stock removals at
`/households/{household_id}/medications/{medication_id}/stock_removals`.
The medication path accepts its numeric or portable identity.

Send `stock_removal` with a positive decimal-string `quantity`, a supported
`reason` and a stable UUID `submission_id`. Optional fields are `note` and
`dosage_id`, the numeric string for a tracked dosage stock source. The API
uses the same stock validation, locking and audit transaction as the web.
It does not create a medication take.

Retry the same submission ID with the same facts to retrieve the original
removal without another stock decrement. Changed facts are rejected.
Permission is checked again before replaying an idempotency-cached response.

History is newest first. Use `page` and `per_page` (maximum 100); the response
includes pagination metadata, saved quantities, reason, note, time and the
actor membership ID when retained. Read access requires the same inventory
management permission as recording a removal.

## Location management

Household managers can create locations and edit or delete them using the
location's current `ETag` in `If-Match`. A missing version returns 428; a
stale version returns 409. Deletion retains both actual administrations and
saved dose decisions. Numeric and portable location identities are accepted.

Create person-location assignments at
`/households/{household_id}/locations/{location_id}/location_memberships`
with `location_membership.person_id` as a numeric or portable person identity.
Repeating the same assignment returns the existing membership. Delete using
the returned membership ID below that collection. These actions require
location-management permission and manage access to the selected person,
including on cached replay. Membership changes remain online-only because
they affect visibility.

## Medicine reviews

The review queue is available at
`/households/{household_id}/medication_review_prompts`. Use `review_status`,
`priority` and `show_hidden` with the same meanings as the web queue, plus
`page` and `per_page` for bounded reads. Each prompt includes its immutable
evidence snapshot and ETag. Responses use `Cache-Control: no-store`.

Read an individual prompt or update it below that collection using its
returned ID. Updates require the current ETag in `If-Match` and manage
access to the prompt's person. Practitioner review statuses require a name,
role and review date. Evidence fields cannot be changed through this API.
The review and its audit event commit together; diagnostics filter private
practitioner details and notes. Current access is checked before cached replay.

## Protected reports

Read typed JSON snapshots at
`/households/{household_id}/reports/health_history` and
`/households/{household_id}/reports/medication_reviews`. Append `.pdf` for
the corresponding binary download operation. Separate operation IDs keep
PDF bytes out of generated clients' JSON response decoders.

Both reports require one `person_id` (numeric or portable), use `no-store`,
and audit successful JSON and PDF downloads. Health history requires manage
access and accepts ISO `start_date` and `end_date` values. Their maximum
difference is 366 days, matching the web GP report. Set
`include_medication_takes=1` to include actual administrations.

Medicine review reports require view access to the selected person and the
existing adult review permission. They show the current visible queue, with
an optional exact `status` filter. They do not accept date filters.
Both formats share the web report queries and PDF renderers. Rendering or
download-audit failures return a stable `report_unavailable` response.
