# OpenAPI-first API checkpoint

Specification: `docs/api/openapi.v1.yaml`, SHA-256 `87714efed85bbad206d1b3361fb5975f3b11f7ce7e58ac4e4f8c22e4a77c41c7`. The operation map is `operations.jsonl`; `operation-filter.jq`, `route-sources.json`, and `test-evidence.json` are its inputs. The map records each operation's path, method, server base, request schema, parameters, effective security, response statuses, response schemas, error component references, and response headers. Rebuild it with:

```fish
yq -o=json '.' docs/api/openapi.v1.yaml | jq -c --slurpfile routes docs/plans/openapi-contract/coverage/route-sources.json --slurpfile evidence docs/plans/openapi-contract/coverage/test-evidence.json -f docs/plans/openapi-contract/coverage/operation-filter.jq > docs/plans/openapi-contract/coverage/operations.jsonl
```

The parsed specification has 118 HTTP operations, all with operation IDs, across 77 paths. It has 227 component schemas and 12 component responses. Global server base is `/api/v1`; 117 operations inherit bearer authentication, and `getCapabilities` is explicitly public. The Rust route scan found 95 documented method/path pairs without a route, 21 with a route but no spec-traced behavioural proof, and two with partial runtime proof. **No operation has complete behavioural coverage.** A route match or an older Rails-derived contract test is not credited as OpenAPI coverage.

## Implemented and tested slice

`listLocations` at [OpenAPI line 991](../../../api/openapi.v1.yaml) and `getLocation` at line 1056 use `LocationCollectionResponse`/`LocationResponse` and `Location` (lines 6001–6031), `PaginationMeta` (line 4325), and the global bearer requirement. The API routes are in `rust/api/src/lib.rs:122-126`; SeaORM reads and location-only pagination validation are in `rust/api/src/read_resources.rs:73,253,303`. Tests are in `rust/contract-tests/tests/openapi_locations.rs`.

The isolated contract tests verify collection and detail 200 responses, required fields, exact JSON object keys, basic numeric ID/UUID/RFC3339 formats, pagination metadata, household scoping, 401 without a bearer, 404 for a foreign location, 422 for six invalid filter values, and a detail ETag. Detail reads accept both numeric and portable IDs. The test evidence file lists the exact checks and omissions for each operation.

Red evidence: the first run returned 404 for both missing routes (`/Users/damacus/Library/Application Support/rtk/tee/1790361517_task_api_e2e3fe.log`, reached through `/Users/damacus/Library/Application Support/rtk/tee/1790361528_task_api_7d11fd.log`). After the read implementation, two tests passed (`/Users/damacus/Library/Application Support/rtk/tee/1790361903_task_api_7d11fd.log`). A new validation test then failed at `page=0`: actual 200, documented 422; later loop cases did not execute in that red run (`/Users/damacus/Library/Application Support/rtk/tee/1790362077_task_api_7d11fd.log`). The final isolated run passed all three tests, including all six invalid filter cases, and removed its owned project, storage, network, volumes and images (`/Users/damacus/Library/Application Support/rtk/tee/1790362281_task_api_7d11fd.log`). `task api:check`, `task api:clippy`, `task api:fmt`, `task api:openapi-locations-fmt`, `task docs:build`, and the independent contract runner test passed. No Rails suite was run because this slice changes no Rails code.

## Remaining gaps and limits

- **Implementation gaps:** 95 documented method/path pairs have no Rust API route. The operation map lists each. Location create, update, delete, and location membership methods remain among them.
- **Untested implementation:** 21 other route matches have no spec-traced behavioural proof. The two location reads have only the listed partial proof; their remaining statuses, valid `updated_since` filtering, later pages, all authorization roles, and 429 `Retry-After` behaviour remain untested.
- **Specification ambiguities:** “visible locations” does not define the role and person-grant rule. The specification documents 429 and `Retry-After` but does not set a rate-limit threshold. These were not inferred from Rails tests or source.
- **Outside this API specification:** Leptos pages and older journey tests were not used as acceptance criteria or changed. Existing database schema and isolated fixture provisioning supplied test records only.

The runner image recompiles broad API and contract-test sources for each isolated run. Its fixture was ready after 41 seconds in the final run; the log does not give a reliable full build duration. A scoped test image is a tooling follow-up, not a reason to weaken the specification tests.
