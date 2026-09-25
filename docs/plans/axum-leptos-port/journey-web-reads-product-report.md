# Shared people and source reads: product preparation

Status: read-only preparation. No Rust production file changed for this tranche; HTTP RED is pending.

## Rails contract mapped

- `PeopleController` index/show use `PersonPolicy::Scope`, which selects people in the requested household with a current `view` or stronger person grant for the active membership. Detail uses numeric ID. The serializer returns portable ID, profile fields, age, ordered location IDs/portable IDs, and notification preference IDs.
- `SchedulesController` and `PersonMedicationsController` index/show use their `current` scopes (`retired_at IS NULL`) and the same view-granted person IDs. A visible paused source remains in the read result. Source detail accepts numeric or portable ID. Both serializers include person/medication portable IDs, decimal dose text, active/paused state, `can_manage` from a current `manage` person grant, and a nullable current pause period. Schedule collection additionally requires an active adult membership.
- Rails collection filtering orders by ID, applies `updated_since` before pagination, clamps page size to 1..100, and returns `data` plus `meta`. Hidden and foreign detail is 404; a valid credential for a foreign household path is 403. Show includes an ETag.

## Intended Rust interface

After a failing HTTP read contract is recorded, add `rust/api/src/read_resources.rs` for six Axum GET handlers and, if SeaORM models make it necessary, `rust/api/src/read_entities.rs`. Handlers will reuse the parent crate's restricted-role transaction, `authenticate`, current membership, person grant query, error envelope, ETag helper, and audit context. Queries will filter by household, current grant and `retired_at`, order and page in PostgreSQL, then serialize the Rails/OpenAPI shapes. No reads will bypass the tenant GUC or RLS boundary.

The owner of `rust/api/src/lib.rs` needs to declare the module and wire the six GET routes for `people`, `schedules`, and `person_medications` index/show. `audit::record_medication_read` is medication-specific, so the parent owner should provide a generic read-audit helper that accepts controller and policy names; alternatively, this module needs an agreed local audit call. No shared file should be edited by this product lane.

## Pending evidence

The frozen `web_reads_api.rs` tests cover three collections, paging/filtering, numeric and portable detail, ETag presence, view versus manage permission, foreign and hidden records, and immediate visibility loss after grant revocation. The fixture correction and first Rust HTTP RED belong to the test writer/runner. Static and combined green checks have not run for this tranche.
