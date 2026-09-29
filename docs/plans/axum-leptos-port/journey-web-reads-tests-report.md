# Shared API read contract test report

Added `rust/contract-tests/tests/web_reads_api.rs` for the browser's shared
`/api/v1` people, schedule, and person-medication reads. The tests use existing
owner and view-only fixture credentials. They check picker fields, decimal dose
strings, `can_manage`, visible paused sources, hidden records, pagination and
timestamp filtering, detail parity, portable source IDs, ETags, and household
denial.

The revocation case inserts a temporary view grant for the already provisioned
hidden person directly into the disposable fixture database. It proves the
person and linked sources become readable over HTTP, marks only that new grant
revoked, then proves all three disappear from the next collection and detail
reads. Cleanup deletes the temporary row, including if an assertion fails. The
existing managed-person grant and API session remain intact. This file does not
cover browser session or dose POST behavior, which have separate owners.

Verification: `rtk task contract:fmt` and `rtk task contract:clippy` passed after
the grant setup correction. The first Rails baseline ran 1/4 because the former
admin grant-create endpoint invalidated the viewer's versioned API session.
The corrected test awaits a new Rails baseline and then Rust RED.
