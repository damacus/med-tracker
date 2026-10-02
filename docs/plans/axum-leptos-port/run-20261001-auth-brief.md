# Authentication parity brief — 1 October 2026

## Objective

Plan complete Rust authentication parity and deliver one safe, independently verified slice: deterministic compatibility of existing Rodauth OTP derivation and acceptance timing, using synthetic data only. Broad parity remains open.

## Ownership

The authentication worker owns `openspec/changes/complete-rust-authentication-parity/`, this brief, the paired research/report, `rust/api/src/auth_compatibility.rs` and `rust/api/tests/auth_compatibility.rs`. The coordinator owns dependency/module registration and task wrappers, review, commits and publication. Shared OAuth routes/session state are untouched by the first slice.

## Accepted policy

Retain account IDs, PostgreSQL compatibility, signed database-backed browser sessions, native account-level S256 PKCE/consent/refresh/revoke, and current household/person permission checks. Default interactive inactivity remains 30 days, with no default absolute maximum. Action-specific fresh-MFA gates stay removed. Login and credential-management factor protections remain.

## Initial acceptance

- A synthetic Rails/ROTP oracle supplies fixed vectors for stored seeds of 16/32 lowercase Base32 characters, HMAC-derived secrets, current/old secret rotation and time-fixed codes.
- Rust RED precedes production helper code; GREEN matches those vectors and rejects malformed keys and invalid/reused/out-of-window codes.
- Accepted-step evidence remains explicit; the helper does not claim DB replay safety without row-lock/update integration.
- No environment secret reads, live credentials, route registration or factor-login enablement.
- Focused task tests, Rust formatting/lint, OpenSpec validation and documentation checks are recorded with exact outcomes.

## Separate gates

Existing 64-byte passkey handles and richer Rust credential metadata need a separate compatibility spike. Factor login requires persistent trusted assurance, transactional OTP/recovery consumption and native/browser acceptance. OIDC and account lifecycle remain later tranches. Stop adding scope at 10:12 UTC and stop safely/report by 10:27 UTC; the coordinator owns final delivery.
