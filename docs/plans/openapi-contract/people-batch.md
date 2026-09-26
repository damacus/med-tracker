# Profile and person batch checklist

Scope: `getCurrentProfile`, `createPerson`, `updatePerson`, and `updatePersonWithPut` in `docs/api/openapi.v1.yaml` at `#/paths/~1households~1{household_id}~1me/get`, `#/paths/~1households~1{household_id}~1people/post`, and `#/paths/~1households~1{household_id}~1people~1{id}/patch|put`. The existing people list/detail routes are read prerequisites. Household API sessions, app tokens and mobile OAuth already use the shared bearer authenticator; SMART grants remain excluded.

## Acceptance checklist for the test lane

- **Profile GET:** exact `MeResponse` and nested `Me`, `MeAccount`, `Person` keys/types; current user and account identity; selected active membership role; a person linked to another household remains the user's own person. Missing/invalid bearer 401, wrong household 403, unknown household 404, shared nonloopback 429. No ETag is declared.
- **Create POST:** strict `PersonCreateRequest` wrapper, required nonempty name and date, optional nullable email, enum person type and boolean capacity; exact `PersonResponse`, 201 and ETag. Owner/admin or an active manage-granted member can create; a member without manage grant gets 403. Valid adult creation normalizes email, gets a Home location and location membership, creates a manage grant and bumps the actor membership's permissions version atomically. Under-age minor and dependent-adult creation sets capacity false; where the issuing membership has a linked carer person, delegation creates the active family relationship and manage grant atomically. Without a carer, no-capacity creation fails 422 without a partial person, location membership or grant. Duplicate/invalid email, age/type mismatch, empty name, invalid date, unknown/extra fields return documented 422. Cross-household and auth/rate cases use shared proof plus endpoint wiring.
- **PATCH and PUT:** both use `PersonUpdateRequest` and merge supplied fields into the existing person, without an If-Match requirement. A person outside the visible grant scope returns 404; a visible person without manage grant returns 403; manage grant succeeds. Test name/email/date/type/capacity validation, normalization, omitted-field preservation, forced false capacity for minor/dependent adult, age/type and carer validation, strict response and ETag, and no partial update/audit/sync event on 422. Both routes cover numeric and portable IDs, missing/invalid bearer, wrong household, and shared 429 wiring. No 409/428 precondition is declared.
- **Side effects and races:** creation and changed updates write person version/sync events and request audit identity. Creation grant changes permissions version, so use disposable credentials and reissue after success; rejected operations leave person, grant, membership version and events unchanged. Two simultaneous duplicate-email creates must not both persist; the loser returns the documented validation status 422. Keep fixtures isolated and clean up.

The source for missing rules is `MeController`, `PeopleController`, `PersonPolicy`, `People::Create`, `CareDelegation::Assign`, `Households::AccessChange::PersonGrant`, `Person`, and the two serializers named in `people-followup.md`. Preserve enum `adult:0`, `minor:1`, `dependent_adult:2`. The profile is the current user's profile even when another household is selected. OpenAPI now declares 400 for malformed JSON or an absent person wrapper and 422 for invalid attributes inside a present wrapper; tests must distinguish them. Do not copy a Rails parameter edge case that rejects a present but invalid wrapper with the wrong status.

The test lane writes independent focused contract cases and proves missing-route or behavior failures before production changes. The production lane owns OpenAPI, Rust API code and any API Task wiring. Shared auth and rate-limit internals need endpoint assertions, not repeated threshold or credential-fuzz suites. The existing evidence map records completion per operation after GREEN.

## Completed batch

All four operations are verified. Final isolated HTTP acceptance passed 17/17
in `mtcontract-eae843f4fe5d408a`; see `coverage/report.md` for the exact log and
shared-proof boundaries. Independent requirements and code review passed after
fixes for carer grant linkage, missing self grants, no-op updates, source email
syntax and concurrent permissions-version increments. No actionable review
finding remains in this batch.

Final checks passed: API format, Clippy, 15 unit tests, selected contract
compilation, People test formatting, runner syntax and dispatch/cleanup tests.
No Rails code changed in this batch. The PR classification defect was fixed
by mapping Rust/contract paths to an actual Rust API/web job. Its local Task
passed, as did all 62 CI-policy tests and classification of every PR path.
Local full actionlint stalled in an external helper and was stopped; focused
validation of the changed workflow with external helpers disabled passed.
This limitation is not reported as a full workflow-lint pass. Remote CI must
confirm the newly published workflow.
