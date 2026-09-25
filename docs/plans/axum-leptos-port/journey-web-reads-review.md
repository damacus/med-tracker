# Shared people and dose-source reads: independent review

Review scope: household people, schedules and person-medication collection
and detail GET routes needed by the first-party medication picker. The
existing bearer and browser session boundaries, direct dose POST, scheduled
occurrence actions and general CRUD remain separate.

## Authority and test baseline

Rails `PeopleController`, `SchedulesController` and
`PersonMedicationsController` use policy-scoped household records, including
current nonretired sources. Their serializers expose portable IDs, person and
medication links, decimal dose strings, pause state and `can_manage` for
source reads. View grants determine visible people and linked sources; source
detail accepts numeric or portable IDs. The root OpenAPI document defines
collection pagination, `updated_since`, detail responses and ETags.

The independent `web_reads_api.rs` contract passed 4/4 against the frozen
Rails authority through its corrected same-namespace sidecar. Against the
unchanged Rust API it failed 4/4 with missing-route 404, establishing RED.
The initial Rails 401 attempt used a faulty runner selection and is superseded
by that corrected 4/4 baseline. The test's temporary grant now uses bound
fixture-database INSERT, revoked-at UPDATE and scoped Drop cleanup, avoiding
unported admin mutation and viewer session-version invalidation.

## Requirements and quality verdict

The compiled source scopes each route by current household, membership and
active person view grants. Schedule and person-medication reads exclude retired
sources, and source `can_manage` uses a current manage grant. Collections filter
before ID ordering and pagination; serializers match the observed Rails picker
fields, including decimal strings, source activity, pause details and the
adult-only schedule index rule. Detail reads preserve the same scoped lookup
for numeric and portable source IDs.

The canonical API snapshot revision two passed 51/51 HTTP cases, including
all six shared-read cases and nine web-session cases. Its frozen full-snapshot
pre/post manifest matched at SHA-256
`7ed2722833a34165d7256d8da2868b56c44e729e6bd17e81c05d7b922f7dc1d7`;
the scoped Rust/runtime manifest matched at
`f823cc917a05639dc6f77be1765fb785eb82b1b02a8208d8949ed4b228b9530d`.
Both disposable projects cleaned up. The four functional
shared-read cases also passed against Rails. Two added 403/422 audit assertions
failed against Rails because `BaseController#audit_api_request` writes inside
`TenantContext.with`'s `requires_new` transaction and re-raises; rollback
removes the audit before Rails renders the error. Rust intentionally corrects
that defect. Its `audited_error_response` writes one redacted event for each
authenticated denial, commits it with OAuth activity, and returns the same
request ID in the error body and header. The focused Rust audit assertions
passed. The detail response's missing `X-Request-ID` matches the existing
Rust medication detail pattern and has no established requirement here.

**Verdict:** the scoped people and dose-source GET requirements are met, with
acceptable code quality. The independent tests cover view-grant revocation,
household/detail privacy, decimal and pause serialization, filtering before
stable pagination, ETags and the corrected failure audit. This is not
acceptance of the medication browser journey or general CRUD. This verdict
uses the final runner's 51/51 HTTP result and reviewed frozen source.

## Bounded cookie-household and API-router split review

I inspected the frozen acceptance snapshot at
`/tmp/medtracker-journey-acceptance-20260925`, not the evolving main UI.
`oauth.rs#households` gives an explicit Authorization header precedence over
cookies, validates a mobile bearer grant or a current DB-backed browser
session, and lists active unrevoked memberships only when their households
are active and operational. The response includes household identity, role
and membership ID without mobile tokens. The browser session path rechecks
account, primary user, factor and session lifetime. `lib.rs#api_router`
keeps every `/api/v1` route, including household discovery, under the shared
cookie CSRF and origin middleware. I found no static blocker in this bounded
split. Its initial 401 RED was closed by the final canonical run's nine
web-session cases, including current cookie household discovery and invalid
bearer precedence. This API route split meets its bounded requirements with
acceptable code quality. Browser journey acceptance remains separate.
