# Shared people and source reads: product report

Status: bounded shared-read API accepted; browser and Leptos UI acceptance are separate work.

## Rails contract mapped

- `PeopleController` index/show use `PersonPolicy::Scope`, which selects people in the requested household with a current `view` or stronger person grant for the active membership. Detail uses numeric ID. The serializer returns portable ID, profile fields, age, ordered location IDs/portable IDs, and notification preference IDs.
- `SchedulesController` and `PersonMedicationsController` index/show use their `current` scopes (`retired_at IS NULL`) and the same view-granted person IDs. A visible paused source remains in the read result. Source detail accepts numeric or portable ID. Both serializers include person/medication portable IDs, decimal dose text, active/paused state, `can_manage` from a current `manage` person grant, and a nullable current pause period. Schedule collection additionally requires an active adult membership.
- Rails collection filtering orders by ID, applies `updated_since` before pagination, clamps page size to 1..100, and returns `data` plus `meta`. Hidden and foreign detail is 404; a valid credential for a foreign household path is 403. Show includes an ETag.

## Rust implementation

The corrected Rails authority run passed 4/4 shared-read cases. The unchanged Rust API then failed all four because the six routes returned 404. `rust/api/src/read_resources.rs` now implements people, schedules, and person-medication collection/detail GET handlers; `rust/api/src/read_entities.rs` supplies their SeaORM read models. The `lib.rs` owner wired the six routes separately.

Every query runs inside the existing restricted transaction with current household membership, RLS context, and current person view grants. Collections filter current, nonretired sources and `updated_since` before count and pagination, sort by ID, and clamp page size to 1..100. Detail accepts numeric IDs for people and numeric or portable IDs for sources. The response fields follow the Rails serializers, including decimal dose strings, pause state, portable IDs, `can_manage` from a current manage grant, ETag, and hidden-record 404. Schedule index uses Rails `adult?` semantics for the member's person.

`rust/api/src/audit.rs` adds a generic resource-read audit helper while preserving the medication wrapper. Authenticated invalid-filter 422 and schedule-index adult-denial 403 now persist one redacted `api.request` audit event and return its request ID in the error body and header. Authentication failures do not use this read audit path. These denial events are an intentional security correction: the Rails implementation rolls its attempted audit back on those errors.

## Evidence and limits

The first compiled implementation passed `task api:check`, `task api:fmt`, `task api:clippy`, `task api:test` (6/6), and focused Rust shared-read HTTP acceptance (4/4). An expanded contract added denial-audit assertions: Rust returned zero audit rows for authenticated 422 and 403, establishing the bounded RED. The separate three 403 failures in that expanded run came from test DOB cleanup and were corrected by the test owner.

The post-fix frozen product files are `rust/api/src/read_resources.rs`, `rust/api/src/read_entities.rs`, and `rust/api/src/audit.rs`. Root copied them with corrected tests to `/tmp/medtracker-journey-acceptance-20260925`. In that snapshot, `task api:clippy` with warnings denied passed, `task api:test` passed 6/6, and `git diff --check` passed (root session 63687, exit 0). The dependency future-incompatibility notice for `proc-macro-error2` is outside the changed code.

Canonical Rust HTTP acceptance passed 51/51, including all six shared-read cases, with task exit 0. The full 3,565-path snapshot and scoped 131-path runtime pre/post manifests matched; exact projects, hashes, and logs are in `journey-web-reads-runner-report.md`. The two Rails denial-audit cases still fail because Rails rolls their audit events back; this Rust correction is intentionally documented. Independent source review passed. Browser and Leptos UI acceptance were not selected in this API run.
