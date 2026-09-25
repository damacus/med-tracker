# Medication read API slice

`task api:acceptance` provisions the existing disposable PostgreSQL 18 Rails
fixture, starts this Axum service against that database, sends focused HTTP
requests to the Rust listener, and removes the isolated Docker project.

The service currently handles collection and single-record medication GETs for
Rails `ApiSession` bearer tokens. It checks the SHA-256 token digest, expiry,
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
The Rails `api.request` audit event, non-`ApiSession` credentials, and other API
routes are not implemented. The service is not a drop-in replacement for the
complete Rails API.

The forecast slice passed eight focused HTTP cases and the conditional-read
slice passed seven, each against the disposable PostgreSQL 18 fixture. A debug
idle sample used 13,232 KiB of resident memory. This does not establish memory
use under representative load or a release-build memory budget.
