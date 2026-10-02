# People ownership brief

Owner: People seat. Scope: list, details, add and edit browser journeys.

Owned production paths are `rust/api/src/web_pages/people.rs` and
`rust/web/src/people.rs`. Shared router, API permissions, rendering shell,
translation catalogue and task changes stay with the coordinator.

The adapter resolves the household using the authenticated cookie session,
reads and writes through existing internal API dispatch, forwards the real
Origin/Referer and CSRF token, and returns private no-store pages. POST handlers
check trusted origin and CSRF before API dispatch. Validation preserves each
native named field as a string. Successful writes redirect to the persisted
person detail, which reads its data again through the API.

Fields: name, email, date_of_birth, person_type (adult/minor/dependent_adult),
has_capacity. Capacity remains false for minors and dependent adults according
to the existing API. API errors are displayed with associated field messages.

Creation permission needs the existing people policy: owner/administrator or
an active manage grant. Editing requires the exact person manage grant.
Affordances must consume shared policy capabilities; they cannot infer a
single manager boolean. Requested coordinator helper supplies these values.

RED gate: no production edits until the test owner records the absent routes
and form behaviour failing. Expected tests cover list/detail/create/edit,
validation drafts, dependent capacity, grants and tenant boundaries. Browser
acceptance and translations require separate evidence before acceptance.

Serena instructions were read; activated language server supports Ruby only.
Targeted Rust reads provide inspection. Current Axum Context7 documentation
confirms Form is the final body extractor and child routers share AppState.
