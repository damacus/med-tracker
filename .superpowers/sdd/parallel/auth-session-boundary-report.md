# Cross-account session revocation contract

## Scope

This isolated branch adds a target-independent HTTP contract for `DELETE /api/v1/auth/sessions/:id` and a focused Rails/Rust Task runner. It updates the parity matrix. There are no Rails product, schema, native client or OpenAPI changes.

## Observed behaviour

- A disposable PostgreSQL 18 fixture supplies separate owner and foreign accounts, each with an API session. The contract discovers the foreign session ID through the foreign account's public session listing.
- Deleting the foreign session with the owner's bearer returns 404. The foreign bearer still lists that session afterward and does not list the owner's session.
- Deleting the owner's session with the foreign bearer returns 404. The owner bearer still lists its session afterward, and its listing does not expose the foreign session ID.
- The existing selected-session contract separately proves that an account can revoke one of its own sessions and that the revoked bearer then returns 401.

The contract checks HTTP status and subsequent bearer validity. It does not assert a structured API error envelope for this controller's scoped `find` failure. The absence of a foreign session from the account-scoped relation produces the same 404 as an unknown ID.

## Verification

- Focused disposable Rails contract: `rtk task contract:auth-session-boundary-rails`, 1 passed, 0 failed.
- Absent Rust target: `rtk task contract:auth-session-boundary-rust` compiled and failed as expected, 0 passed and 1 failed at `127.0.0.1:39999` because no Rust server is running.
- Static checks: `rtk task contract:fmt`, `rtk task contract:clippy`, `rtk proxy fish -n rust/contract-tests/run.fish`, and `rtk git diff --check` passed. `rtk task docs:build` could not open the host uv cache under the sandbox; the installed Zensical executable built the Markdown with `No issues found`.

No full contract corpus was run in this isolated branch; the integration owner runs the combined suite.
