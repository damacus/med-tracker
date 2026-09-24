# API black-box cases

Run `task contract:rails` to start a unique test-only Compose project with its
own PostgreSQL 18 container, `medtracker_contract` database, migration, and Rails
server. It creates disposable households, sessions and an OAuth public client,
then runs the black-box cases. The project name is recorded in the run directory
under `tmp/contract-tests/`. Fixture JSON has mode `0600`. On success or failure,
the harness validates its ownership marker and uses `task contract:cleanup` to
remove only that project's containers, network, volumes and generated web image.
Run `task contract:isolation` to check cleanup exit statuses and then verify
two simultaneous projects have separate fixture rows and cleaning one leaves
the other healthy.
Rack::Attack is enabled only for this contract test server so the 429 response
can be observed over HTTP.
The contract test environment raises only the global request-per-IP ceiling to
3000 so the complete suite can finish in one five-minute window. The targeted
data-export ceiling remains 10 per minute, and production keeps its 300-request
global ceiling.

Run `task contract:dosage-health-rails` for only the dosage-option and
health-event black-box cases with the same isolated fixture setup.
Run `task contract:admin-rails` for household settings and membership cases.
Run `task contract:admin-rust` to check the same cases against a local Rust
target after provisioning the isolated fixtures.
Run `task contract:schedules-rails` for only the schedule cases.
Run `task contract:doses-rails` for schedule and direct-assignment dose outcomes
and medication takes.
Run `task contract:reviews-rails` for medication review prompt list, read and update cases.
Run `task contract:care-rails` for people, person-grant, and location permission cases.
Run `task contract:sync-rails` for sync v2, mobile v1, and change-feed cases.
Run `task contract:replay-rails` for separate medication-take `client_uuid` and
`Idempotency-Key` replay cases, including cached-success permission rechecks
after an owner revokes the exact person grant. The disposable fixture includes
a delegated mobile OAuth bearer credential that remains valid after the grant
change; its older API session is expected to return 401 because the membership
permission version changed.
The replay file also submits paired sync batches through independent HTTP
clients released by a barrier. It checks keyed response replay, persistent
take UUID replay across different or absent request keys, and conflicting edits
with distinct keys. The barrier aligns client dispatch; server scheduling may
still serialize the requests, so this run does not prove internal overlap.
Both requests in each pair use the same fixture-scoped `X-Forwarded-For` value
on the local target to keep the 30-per-minute sync limit independent of other
contract cases. A future Rust target may need equivalent trusted-proxy test
setup or another isolated rate-limit bucket.
Run `task contract:sync-privacy-rails` to demonstrate the ignored hidden-person
feed defect: Rails exposes a hidden health-event change and a hidden
person-medication tombstone to a member without that person's grant. The probe
captures a snapshot cursor before the writes and deletes the assignment through
the public sync batch route. It recursively checks response fields for hidden
portable IDs and reports only disclosure booleans. It also checks a visible
managed event and pause period, plus hidden pause exclusion.
Run `task contract:sync-rust` to record expected connection failures until the Rust API exists.
Run `task contract:dose-precision-rails` to demonstrate the ignored
expected-failing medication-take timestamp case: Rails accepts fractional
seconds but returns `taken_at` at whole-second precision.
Run `task contract:schedule-precision-rails` to demonstrate the ignored
schedule case that currently fails on Rails: `"8.5"` minimum hours returns
`"8.0"`. This expected failure is outside the green parity suite.
Run `task contract:dosage-health-privacy-rails` to demonstrate the separate
ignored privacy case that currently fails on Rails: a viewer can list a dosage
option for a medication without a person grant. This expected failure is not
part of the green parity suite.

Run `task contract:oauth-rails` for only the OAuth black-box cases with the
same isolated fixture and server setup.

Run `task contract:rust RUST_URL=http://127.0.0.1:39999` to run the same cases
against a Rust server. Until that server exists, connection failures are
expected. Fixture creation still uses isolated Rails; an external Rust server
cannot see those fixture rows unless it uses this run's database or provisions
equivalent records itself. Each URL is checked by
the Rust runner before any request. Loopback HTTP(S) origins are accepted.
For a remote HTTPS origin, pass its exact value as `APPROVED_ORIGIN` to
`task contract:run` with an existing fixture path. The runner rejects redirects
and ignores HTTP proxy settings.

`task contract:run BASE_URL=http://127.0.0.1:3000 FIXTURE_PATH=/absolute/path`
runs against a prepared target and fixture. Its fixture must contain real
bearer tokens and IDs for separate households in that target's own disposable
environment. The runner rejects every DELETE, PATCH and POST request to a
non-loopback target, even when that target has an approved HTTPS origin.
