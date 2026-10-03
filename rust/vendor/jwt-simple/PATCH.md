# Optional RSA dependency patch

This is the crates.io `jwt-simple` 0.12.17 release, licensed under ISC.
Its source, README and licence are retained; Cargo.toml is the release's
Cargo.toml.orig. The API uses this copy through `[patch.crates-io]`.

The local patch adds an `rsa` feature and gates the RSA module, exports and
RSA-only tests/constants. `pure-rust` no longer enables `superboring` by itself;
`rsa` enables it. Default `optimal` and `jwe` features retain RSA support.
The Web Push dependency requests only `pure-rust`, with defaults disabled,
so it compiles unchanged ES256 code without `superboring` or RustCrypto `rsa`.
The wasm dependency is also optional. No cryptographic implementation or
Web Push protocol code is changed.

Release archive: <https://crates.io/api/v1/crates/jwt-simple/0.12.17/download>
SHA-256: `06a4207b9d1f423858853fb9ee8b34aebc2b661f3b446ca4efda46c4b3700a2f`
Security reason: <https://rustsec.org/advisories/RUSTSEC-2023-0071.html>

Remove this patch when an upstream release supports disabling RSA. Upgrading
the upstream release requires reapplying and reviewing these feature gates.

Verification: `task api:vendor-jwt-test` runs the upstream non-RSA library tests.
API tests independently verify VAPID signatures and decrypt the encrypted
payload. The API lockfile regression rejects reintroducing the `rsa` crate.
