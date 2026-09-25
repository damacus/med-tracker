# Medication read API slice

`task api:acceptance` provisions the existing disposable PostgreSQL 18 Rails
fixture, starts this Axum service against that database, sends focused HTTP
requests to the Rust listener, and removes the isolated Docker project.
The API and HTTP test process run inside that project's Compose network. The
test sidecar shares its API container's network namespace and uses loopback;
the API publishes no host port. Each run has its own fixture,
database, network and tagged test image. The test image caches dependency
compilation separately from application source.

The service currently handles collection and single-record medication GETs for
Rails `ApiSession` and mobile OAuth bearer tokens. It checks the token digest, expiry,
revocation, verified account, active user and membership, permissions version,
household state, and medication visibility. Every read uses a transaction with
the restricted `med_tracker_app` role and transaction-local tenant settings.
The connection pool has a maximum of four connections. Delegated visibility is
expressed as ORM subqueries, so pagination does not first load all linked IDs
into application memory.

Collection reads apply `updated_since` to the scoped ORM query before count and
pagination. Single-record reads return an ETag derived from the serialized
response. A matching `If-None-Match` yields an empty 304 response after
authorization, scoped record lookup, and serialization. Rails currently ignores
that conditional request. Rails also hashes only the medication's `updated_at`
for its ETag, which can stay unchanged when schedule, assignment, location, or
time-dependent forecast fields change. The Rust ETag deliberately differs so a
304 cannot validate a stale representation.
Future Rust write endpoints must validate `If-Match` against this GET tag;
write-side concurrency and compatibility with Rails record-only tags remain
unimplemented.

This is a focused response subset. It returns core identity, inventory,
location, stock flags, and collection pagination fields. Stock forecasts use
the Rails daily consumption rule for active schedules and person medication
assignments. Related rows are fetched in bounded batches for each response
page. The forecast uses UTC by default and honours an explicit IANA `TZ` zone.
Mobile OAuth uses the padded URL-safe Base64 SHA-256 digest, exact `medtracker`
scope, login inactivity and optional maximum-age limits, and current household
membership. Read audits identify `oauth_grant:<id>` without storing bearer
material. Valid mobile credentials refresh activity on successful reads,
household denial, and an invalid `updated_since` filter; database or audit
failures roll back the transaction. Invalid lifetime environment settings deny
OAuth authentication. The disposable acceptance API enables a 30-day maximum
login age to exercise that policy; the product default remains unlimited.

`ApiSession.touch_last_used!`, app/integration credentials, OAuth issuance, and other API
routes are not implemented. The service is not a drop-in replacement for the
complete Rails API.

Authenticated medication list and show reads write `api.request` rows through
SeaORM under the restricted role. Successful reads use a `success` outcome; a scoped
record miss records a 404 `failure`; rejected sessions and household bindings
do not write a request event. Audit insertion and the read share a transaction,
so an audit insertion failure yields a server error and rolls back the request.
The current slice generates a request ID for the audit row but does not yet
return a matching `X-Request-Id` response header. Transport IP and exception
`error` outcomes remain to be ported. Other post-authentication failures,
including invalid `updated_since` filters, are not yet audited. An authorized
conditional 304 writes a successful read event before committing.
The disposable fixture is loaded from `db/schema.rb`, which does not recreate
the migration-defined audit ledger trigger and view. The focused test verifies
the audit source row directly; ledger append still needs a migrated-database
check.

Two concurrent isolated Compose runs each passed 19 of 19 focused HTTP cases
against PostgreSQL 18: seven mobile OAuth, nine medication read, and three
forecast cases. Docker's default address pool was exhausted on the development
host, so these runs used distinct checked /28 subnet overrides. The optional
`CONTRACT_TEST_SUBNET` override currently supports macOS host-route checks;
ordinary runs retain automatic Compose allocation.

Before the OAuth/container changes, a debug idle sample used 13,376 KiB of
resident memory. This is historical API-only evidence and does not establish
the current release-build, workload, or combined web/API memory budget.
