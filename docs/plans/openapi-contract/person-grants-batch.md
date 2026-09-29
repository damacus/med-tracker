# Person access grant administration

Implement listPersonAccessGrants, createPersonAccessGrant and
deletePersonAccessGrant after token and membership administration. Prepare
compiled black-box tests in parallel, then prove HTTP RED before production.

Owners and administrators manage grants within the selected household. Validate
membership, person and grantor household boundaries. Preserve view, record and
manage levels and the documented relationship types. Reject unknown fields and
invalid scalar values without changing access. A valid past expiry creates an
inactive grant; do not invent a future-only model validation. Unrevoked grants
remain unique per membership/person pair even when expired.

List includes revoked and expired grants with the exact response shape. Revoke
preserves the first revocation timestamp, avoids repeated permission-version
increments and records the expected no_change audit outcome. Relationship-owned
grants must be revoked through their relationship and return 422 here.

A real grant creation or revocation increments the affected membership's
permissions_version, invalidating stale credentials. Preserve the redacted
household_access.person_grant_changed audit shape and success, no_change and
rejected outcomes. Reauthenticate under the household lock and verify shared
idempotency without duplicate grants or permission increments.

Use disposable or restored fixture records, test relevant concurrent creation,
tenant boundaries and all three rate-limit paths, and publish only after
independent review and isolated acceptance. Sol owns production and shared-file
integration; Luna owns openapi_person_grants.rs.
