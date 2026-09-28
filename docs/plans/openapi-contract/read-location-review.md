# Read and location completion review

## Scope

The current tranche completes the fixed OpenAPI capabilities, household and
people reads, and six location mutations. Existing location collection/detail
handlers remain unchanged; their prior isolated acceptance passed eight read
tests (`location-writes-review.md`). This tranche adds strict query errors,
account and household credential scope, idempotent location writes, duplicate
name validation, domain versions, sync changes, and deletion tombstones.

## Evidence

The first valid-path read/location HTTP run established a production RED.
After the cascade regression exposed missing child tombstones, the final
isolated run (`mtcontract-f3f5dd6e397d4c14`) passed 20/20 location-write
tests and 3/3 read-completion tests. The cascade case verifies source person
metadata, medication visibility IDs, and a location tombstone. The project
removed its containers, network, images, volumes, and fixture storage. The
selected test compiled; `task api:check`, `task api:fmt`, and
`git diff --check` passed before the run.

## Contract decisions

An app token lists only its issuing household. This corrects Rails account
metadata exposure for an account belonging to more than one household.
Mobile OAuth and API sessions list the account's households; integration
OAuth cannot use this account-level endpoint. Authorized validation and
precondition failures are cached under idempotency keys with a fresh request
audit and request ID on replay. A changed request using the same key returns
409. Duplicate location names return 422.

Capabilities report unavailable Rust FHIR, MCP, backup, sync, report, and CLI
features as unsupported until those handlers exist. This avoids advertising
endpoints outside the completed Rust surface. As those families land, their
flags require another capability review.

Location deletion retains history when required. When deletion is allowed,
child deletes and their version/tombstone effects occur in one transaction.
Association-driven membership deletion does not emit an extra Person sync
change; direct membership deletion does.
