# API black-box cases

Run `task contract:rails` to start this worktree's isolated Rails test server,
create two disposable households and a normal bearer session, and run the three
initial cases. Each run stores fixture JSON in its own directory under
`tmp/contract-tests/` with mode `0600` while the command runs, then removes
that directory. The test database is local to this worktree's Docker Compose
project; records remain there until that disposable test database is removed.

Run `task contract:rust RUST_URL=http://127.0.0.1:39999` to run the same cases
against a Rust server. Until that server exists, connection failures are
expected. Fixture creation still uses isolated Rails. Each URL is checked by
the Rust runner before any request. Loopback HTTP(S) origins are accepted.
For a remote HTTPS origin, pass its exact value as `APPROVED_ORIGIN` to
`task contract:run` with an existing fixture path. The runner rejects redirects
and ignores HTTP proxy settings.

`task contract:run BASE_URL=http://127.0.0.1:3000 FIXTURE_PATH=/absolute/path`
runs against a prepared target and fixture. Its fixture must contain a real
bearer token and IDs for two separate households in that target's own
disposable environment. The runner sends only GET requests.
