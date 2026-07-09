# API Backlog Security Review

## Executive Summary

| Field | Value |
|-------|-------|
| Application | MedTracker |
| Review Date | 2026-07-07 |
| Reviewer | Codex |
| Scope | Local Rails API implementation for issues #551, #1480, #1485, #1486, #1487, #1488, #1489, and #1491 |
| Overall Risk Level | Low after local verification, with accepted scanner caveats |

This review is limited to the local repository, local development/test services, and GitHub CI evidence. It does not authorize production penetration testing, destructive testing, data exfiltration attempts, or testing against third-party services beyond mocked/local request specs.

## Attack Surface

The implementation touches these security-critical paths:

- API bearer authentication, API app tokens, OIDC exchange, refresh, revocation, and household session selection.
- Household-scoped API resource reads and writes, including portable ID lookup and cross-household rejection.
- Batch mutation, sync, tombstone, backup, export, import, and FHIR read endpoints.
- Owner/admin household administration endpoints and fresh MFA or upstream OIDC MFA proof checks.
- Logs, OpenTelemetry spans, Rack::Attack throttle responses, and security audit metadata.

## Automated Scan Results

| Tool | Command | Status | Notes |
|------|---------|--------|-------|
| Brakeman | `task brakeman` | Pass | Brakeman 8.0.5, Rails 8.1.3, 0 warnings. A medium `permit!` finding in the API medication reorder endpoint was fixed with explicit permitted keys before the passing run. |
| Bundler Audit | `bundler-audit check` | Pass | Host tool reported no vulnerabilities. `task test:exec CMD='bundle exec bundler-audit check'` could not run because the test container does not include the executable. |
| Gitleaks | `gitleaks detect --source . --verbose` | Reviewed findings | History scan reported five findings in a deleted `_bmad` commit: one expired example JWT and four manifest hashes. Working-tree scan with `gitleaks dir . --verbose` reported ignored runtime files under `log/` and `tmp/`, primarily test token digests, idempotency keys, and Bootsnap cache strings. No current tracked application source finding was identified. |
| Semgrep | `semgrep --config=auto .` | Tool failure | Semgrep was installed but failed before scanning with `Failed to create system store X509 authenticator: ca-certs: empty trust anchors`. No Semgrep result was available. |
| RuboCop | `task rubocop` | Pass | 1354 files inspected, no offenses detected. |
| RSpec | `task test` | Pass | 3821 examples, 0 failures, 1 expected pending OIDC configuration example. |

## Manual Review Checklist

- [x] Authentication rejects missing, expired, revoked, locked, stale-permission, and cross-household credentials.
- [x] Authorization reuses Pundit policy scopes for every API read and write.
- [x] Privileged household administration mutations require fresh local MFA/passkey or upstream OIDC MFA proof.
- [x] Portable ID lookup never leaks whether an inaccessible cross-household record exists.
- [x] Idempotency storage does not persist request bodies, PHI, passphrases, tokens, authorization codes, or backup contents.
- [x] Batch mutation failures roll back every write in the batch.
- [x] Portable exports exclude sessions, tokens, device tokens, push subscriptions, invitations, support sessions, audit internals, platform state, and Rails numeric IDs.
- [x] FHIR endpoints expose only policy-authorized resources.
- [x] Rate-limit responses include retry guidance without leaking sensitive request context.
- [x] Logs, spans, errors, and security audit metadata redact PHI and secret material.

## Findings

No confirmed high-severity findings in the implemented local API backlog pass.

- OIDC exchange uses the existing internal API session bearer-token model. It does not introduce Rodauth provider-side OAuth.
- Privileged household administration API writes require fresh API-session OIDC MFA proof and write redacted security audit events.
- Idempotency stores request digests and response envelopes for replay. It does not store raw request bodies.
- API sync safety tables are household-scoped and included in the schema inventory contract.
- The Brakeman mass-assignment warning on API reorder details was remediated by replacing `permit!` with explicit keys: `supplier`, `quantity`, and `expected_arrival_on`.

## Accepted Risks

- Full external OIDC issuer key discovery is not exercised in local tests; this pass validates the local exchange contract and records the hosted-OIDC direction for CI/provider integration follow-up.
- Gitleaks history findings are accepted as pre-existing non-current-source noise from a deleted `_bmad` commit. Current-tree findings are ignored runtime artifacts in `log/` and `tmp/`.
- Semgrep produced no scan result because the local installation failed during CA trust-store initialization before rule execution.
