# API black-box cases

Run `task contract:rails` to start this worktree's isolated Rails test server,
create disposable households, sessions and an OAuth public client, and run the
black-box cases. Contract data lives in the separate `medtracker_contract`
database in this worktree's test PostgreSQL container, leaving the ordinary
Rails test database available for RSpec fixtures. Each run stores fixture JSON
in its own directory under `tmp/contract-tests/` with mode `0600` while the
command runs, then removes that file. Contract records remain only in the
disposable contract database until the test database volume is removed.
Rack::Attack is enabled only for this contract test server so the 429 response
can be observed over HTTP.
The contract test environment raises only the global request-per-IP ceiling to
3000 so the complete suite can finish in one five-minute window. The targeted
data-export ceiling remains 10 per minute, and production keeps its 300-request
global ceiling.

Run `task contract:dosage-health-rails` for only the dosage-option and
health-event black-box cases with the same isolated fixture setup.

Run `task contract:oauth-rails` for only the OAuth black-box cases with the
same isolated fixture and server setup.

Run `task contract:rust RUST_URL=http://127.0.0.1:39999` to run the same cases
against a Rust server. Until that server exists, connection failures are
expected. Fixture creation still uses isolated Rails. Each URL is checked by
the Rust runner before any request. Loopback HTTP(S) origins are accepted.
For a remote HTTPS origin, pass its exact value as `APPROVED_ORIGIN` to
`task contract:run` with an existing fixture path. The runner rejects redirects
and ignores HTTP proxy settings.

`task contract:run BASE_URL=http://127.0.0.1:3000 FIXTURE_PATH=/absolute/path`
runs against a prepared target and fixture. Its fixture must contain real
bearer tokens and IDs for separate households in that target's own disposable
environment. The runner rejects every DELETE, PATCH and POST request to a
non-loopback target, even when that target has an approved HTTPS origin.
