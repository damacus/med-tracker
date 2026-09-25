# Location completion tranche

Source: `docs/api/openapi.v1.yaml` operations `listLocations`, `getLocation`, `createLocation`, `updateLocation`, `updateLocationWithPut`, `deleteLocation`, `createLocationMembership`, and `deleteLocationMembership` at lines 991–1272. The read pair was partially implemented in the preceding checkpoint. This tranche adds remaining read checks and documented writes only; no UI, Rails behaviour, or spec edits.

Test-first evidence: seven new write tests compiled and failed against missing Rust methods in isolated project `mtcontract-fb935387425f4a2a`. Location POST/PATCH/PUT/DELETE returned 405; membership POST/DELETE returned 404. Concurrent PATCH returned `[405, 405]`. See `/Users/damacus/Library/Application Support/rtk/tee/1790365256_task_api_7d11fd.log`. Additional strict-schema, null/absent, and foreign-household assertions were added after this missing-route RED and have not yet run. One consolidated green run will test them after implementation.

The write tests exercise request/response schemas, 201/200/204 results, JSON errors, owner bearer access, ETags, missing and stale `If-Match`, atomic same-ETag updates, foreign-household nonmutation, membership replay and location-scoped deletion. Fixture SQL supplies disposable rows and cleanup only; it is not a behaviour source. API reads and writes will use SeaORM.

Pending policy questions from OpenAPI wording: which roles and person grants make a location visible or a person “managed”; which retained history blocks location deletion; rate-limit thresholds. Until clarified, write access is limited to the owner fixture rather than opened to every authenticated member. No role/retention policy is inferred from Rails.
