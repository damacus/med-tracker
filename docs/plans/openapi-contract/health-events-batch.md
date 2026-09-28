# Health event contract tranche

Cover the five fixed operations in `docs/api/openapi.v1.yaml`: `listHealthEvents`,
`createHealthEvent`, `getHealthEvent`, `updateHealthEvent`, and
`replaceHealthEvent`. Keep the operation inventory unchanged. The initial
missing-route HTTP RED preceded implementation. Runtime evidence and remaining
checks are recorded in `final-families-review.md` and the coverage ledger.

Existing acceptance proof is concentrated in
`rust/contract-tests/tests/dosage_health.rs`: portable person/medication links,
collection paging and `updated_since`, get/create/update/replace ETags, stale
write conflicts, validation and cross-household denial, hidden-record filtering,
view versus manage grants, and request audit. `rust/contract-tests/tests/sync.rs`
separately proves that event create/update produce the expected change-feed
entries. Reuse these targets; do not repeat the broad relation, grant, audit, or
sync scenarios in this tranche.

The new focused target adds exact response field/type checks, forbidden extra
request keys and invalid create/update payloads with state/ETag nonmutation,
and 429 response checks for all five documented rate-limited operations. Run it
with the existing `dosage_health` and `sync` targets for selected verification.
OpenAPI does not declare an `Idempotency-Key` parameter for these operations,
so this tranche makes no idempotency claim.

Known Rails reference failure: `health_event_invalid_kind_returns_validation_error`
previously had to be ignored because an unsupported `event_kind` raised 500
instead of the documented 422. It is enabled for Rust and passes with the 422
expectation; invalid input must not write an event.

The update schemas have the same optional fields for PATCH and PUT. Following
the user's partial-update default, omission of `medication_ids` on either
method preserves existing links; an explicit empty array clears them. The
focused test covers both methods. `HealthEventAttributes` also permits
`person_id`, but Rails removes it in `health_event_update_params`. The user
approved reassignment when the caller can manage both people. Tests must cover
both methods, denied targets without mutation, old/new sync visibility, and
batch replay after the original person's grant is revoked.

Schema-invalid null/unknown values silently filtered by Rails are rejected in
Rust. Independent review and isolated acceptance are required before ledger
credit; known Rails defects are not preserved as the desired behaviour.
