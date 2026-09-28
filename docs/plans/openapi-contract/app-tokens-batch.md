# App token administration

After the medication families, implement listAppTokens, createAppToken and
deleteAppToken. Prepare tests in parallel using existing admin.rs scenarios,
but require initial HTTP RED before production implementation.

Owners and administrators manage their own account's tokens in the selected
household. Ordinary members cannot use these administration routes; another
account's or household's token remains undisclosed. List responses never expose
token secrets or digests. Creation returns a cryptographically random secret
once, stores its SHA-256 digest and captures the current permission version.

Preserve configured calendar-month maximum lifetime and validate explicit
expiry after issuance and within that maximum. Reject blank names, forbidden
nulls and unknown request fields. Duplicate display names are not inherently
duplicate credentials; follow model rules rather than inventing uniqueness.

Revocation invalidates bearer authentication immediately. Reuse current
membership/permission-version checks and verify fresh authority on replay.
Audit creation and revocation without secret material. Repeated revocation
returns 204 without a duplicate domain audit; list includes revoked tokens.

Correct Rails' plaintext caching of the one-time token response. A keyed create
returns the secret only on the first successful response and stores a nonsecret
receipt. Repeating the same key returns 409 with a clear token_already_issued
error and does not issue another credential. A changed payload also conflicts.
This deliberate exception to ordinary response replay preserves one-time secret
delivery; neither idempotency storage nor audit records may contain the raw
token. A caller needing a replacement must explicitly use a new key. Ordinary
shared replay remains suitable for revocation.

Check exact documented response shapes and error envelopes, endpoint rate
limiting and invalid-input nonmutation. Any missing documented validation
response must be reported as a contract clarification, without changing the
fixed operation inventory or claiming an undocumented response is documented.

Luna owns openapi_app_tokens.rs; Sol owns production and shared integration.
Independent review, isolated acceptance and publication precede ledger credit.
