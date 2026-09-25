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

This is a focused response subset. It returns core identity, inventory,
location, stock flags, and collection pagination fields. Forecast values are
currently null, including when Rails would calculate non-null values. The
`updated_since` filter, show ETag, Rails `api.request` audit event, non-`ApiSession`
credentials, and other API routes are not implemented. The service is not a
drop-in replacement for the complete Rails API.

The focused Rust-server acceptance run passed 5 of 5 HTTP cases against the
disposable PostgreSQL 18 fixture. Its debug build used 11,984 KiB of idle
resident memory after startup. This single idle sample does not establish
memory use under representative load or a release-build memory budget.
