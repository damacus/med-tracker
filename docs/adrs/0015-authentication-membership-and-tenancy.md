# ADR 0015: Authentication, membership and tenancy responsibilities

- Status
  Accepted direction; implementation requires verification
- Date
  2026-10-21

## Context

MedTracker needs maintained authentication protocols and one authority for
household membership. The previous integration implemented Better Auth's storage
interfaces itself and used organisation adapters for application membership.
That approach added code which MedTracker would have to maintain.

The owner approved replacing the authentication schema and endpoint wiring with
Better Auth's native schema and plugins. Authentication data compatibility is not
required. This does not authorise resetting a populated database or deploying the
replacement.

## Decision

Use the published Rust `better-auth`, `better-auth-core` and
`better-auth-seaorm` releases at `1.0.0-alpha.5`. Package dependencies use released
versions, without Git SHA pins, unreleased branches, vendored authentication code
or local patches to the library.

Use Better Auth's SeaORM store and supported plugins for passwords, email
verification and password recovery, sessions, passkeys, MFA, provider sign-in and
personal API keys. Default to stock Better Auth behaviour and native credential
management when provided, including native current-password confirmation for
disabling MFA. Remove the custom authentication store and historical
credential, token and date compatibility. Application transport may adapt native
endpoints to existing pages; it must retain CSRF, origin and authentication checks, native protocol
checks and all response cookies.

Map the native User to the canonical `accounts` table and native provider Account
to `auth_credentials`. Each account identifies exactly one Person directly.
The fresh Rust schema has no separate `users` or clinical-profile table.
Account activation belongs to `accounts.active`; sign-in, retained sessions,
API access, guardian eligibility and support approvals check that state.
Clinical and household audit attributes the Account and Person, with the Account
ID in `whodunnit`. Member administration identifies a Membership directly.
This fresh-schema cleanup is required before 0.6 and requires no data migration
or compatibility layer. Existing populated databases must not be reset or adopted.

Preserve required passkey user verification through the public native
after-verification hooks, using the library's verified metadata. Request required
user verification and discoverable credentials through standard browser options.
Do not copy WebAuthn parsing or verification to enforce application policy.

Use native MFA backup codes after the first sign-in step and native email/password
recovery. Standalone sign-in with a recovery code is deliberately removed, as
approved by the owner. Personal API keys remain a separate feature. Their creation
uses the native authenticated-session flow, with application household scopes and
permissions, without an additional application proof ceremony. Key lifecycle
notices and subject audit remain atomic with the native database write.

Authentication audit may record the affected account or session atomically, with
trusted HTTP actor and request events recorded separately. Clinical and household
audit requirements are unchanged. Do not add custom authentication storage merely
to reproduce the previous audit linkage.

Authentication proves account identity. Application membership decides household
participation; disclosure authority decides cross-household sharing. Keep these
rules in existing domain operations and PostgreSQL RLS. Do not mount a second
organisation membership authority. Session household selection cannot grant
membership. Every clinical request and mutation rechecks current authority.

Keep domain transaction boundaries, including invitation acceptance, account
closure, mail outbox, audit, replay protection and stock deduction. Compose
ordinary domain writes with supported native transaction hooks where required;
do not introduce another authentication-store proxy or transaction framework.

Account-profile access does not grant global clinical-history access. A global
self-history reader derives the subject from the authenticated account and
accepts account-bound first-party mobile OAuth or a maintained browser session.
Household-bound API sessions, app tokens, personal keys and integration
credentials cannot use it to read across origin households. Their existing
household APIs and restrictions remain in force.

Leaving the last household does not remove an account holder's own history
access. A dependent adult with a legitimate account also retains self access;
capacity rules for sharing or editing another person's care do not replace
self identity. Historical reads preserve original attribution and require a
separate account-owned clinical-access audit, without inventing a household.
These are implementation constraints, not evidence that the reader is complete.

The browser reader uses `/profile/history`; account-bound first-party mobile
OAuth uses `/api/v1/auth/profile/history` with the same narrow taken-dose reader.
Both use bounded taken-time and ID pagination. Successful reads commit their
account-owned audit before releasing records. Refused reads identify the actor
only when the server has proved their identity; otherwise the refusal has no
claimed actor. Do not retain additional session records solely to identify
requests made after sign-out. This decision does not introduce the deferred
download product described in ADR 0017.

Refusals without a proved identity use the existing structured security logs,
under the server-generated request trace. This includes anonymous browser
requests, invalid credentials, rate limits and missing socket peers. Do not write
anonymous refusals to the database audit ledger or log supplied credentials,
query values or client request IDs. Keep the limiter's rejection and retry
behaviour; do not add database work after throttling. These refusal logs do not
have the clinical ledger's transactional or tamper-evident guarantees. Successful
reads and refusals by callers whose identity has been proved use that ledger.
For `AccountClinicalAccess` version events, the ledger envelope takes its outcome
from `audit_context.outcome`. Security events retain `metadata.outcome`, and
other version events retain their existing capture behaviour. The forward
migration changes future entries without rewriting retained ledger history.

## Consequences

Native authentication tables use restricted runtime grants and the native store's
endpoint access controls, without the old adapter's per-account authentication-table
RLS. The native store persists its session tokens directly; the old custom digest
representation is removed. Verify cross-account HTTP denial and keep usable tokens
out of inventories, errors and logs. Audit references remain digested. Clinical
and household data retain their restricted runtime roles and RLS checks.

Verify real signup and sign-in, native recovery, passkeys, MFA, provider callbacks,
session expiry and revocation, personal-key permissions, cross-account denial,
replay and domain rollback using the application and disposable PostgreSQL.
Architecture acceptance alone does not prove feature preservation or completion.
