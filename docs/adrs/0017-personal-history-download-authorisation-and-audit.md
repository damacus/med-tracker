# ADR 0017: Personal-history download authorisation and audit

- Status
  Accepted (design; implementation and verification pending)
- Date
  2026-10-15

## Context

Personal medication history remains available after household departure. Preparing
a large download must use bounded memory and must not keep an authorisation
transaction open while a client receives the file.

The maintained Better Auth session store deletes revoked sessions. A later denial
audit cannot reconstruct the deleted session's ownership from its absence or from
the existing generic revocation event. Retaining that historical proof must not
create another authentication authority.

## Decision

Use three stages for a personal-history download:

1. Validate the maintained browser session with Account SHARE before the exact
   session lock. Start the consistent snapshot at transaction creation, before
   validation queries. Read immutable history in bounded pages into a private
   temporary file. Release no clinical bytes during preparation.
2. After the file is prepared, append a fixed
   `self_medication_history.download.prepare` event in that snapshot transaction.
   Derive Account, Person and canonical session identity from validated server
   context. Commit the preparation and its ledger entry together. Keep its ID,
   exact identity and server request UUID in a private typed handle.
3. Open a fresh transaction. Lock Account before validating the same maintained
   session again, and require the captured Account, Person and session to match.
   Commit the terminal access audit before releasing the file through a bounded
   stream. Do not hold database locks during delivery.

Preparation records prove historical attribution only. They never grant access,
validate a session or replace a revocation check. A fixed denial writer can use
that immutable proof after the session has been deleted, matching the preparation,
Account, Person, canonical session and server request UUID. Keep maintained
session-deletion behaviour unchanged.

Use the existing account-owned clinical-access audit source. Preparation has an
explicit `prepared` outcome and no HTTP status. Store no clinical values, raw
credentials, temporary-file paths or file contents. Limit owner SELECT access to
preparation events; application code retains no raw source-table access. Link a
terminal success or denial through a typed preparation reference and enforce at
most one terminal outcome per preparation in PostgreSQL.

The version 1 download envelope contains `format`, `version`, `exported_at` and
`data`. Its format is `medtracker.personal_medication_history`; its data contains
the same allowlisted administration records as the self-history API. This is an
administration-history download, not a full personal narrative export or a
MedTracker migration package.

## Consequences

A preparation may remain without a terminal outcome after cancellation, a crash
or a failed final audit. It does not mean data was delivered. A successful terminal
event means release was authorised, not that the client received every byte.
Temporary files are removed on failure or cancellation. Audit failure releases no
clinical data, even when it prevents returning an attributable authentication
denial. Audit retention remains governed by the existing audit policy.

Acceptance needs real maintained-session revocation, both release/revocation
orderings, exact-credential matching, audit rollback, duplicate terminal rejection,
complete multi-page output and unchanged clinical state. This ADR does not claim
those checks have passed.

Related decisions: [Household inventory and personal care](0014-household-inventory-and-personal-care.md)
and [Authentication, membership and tenancy](0015-authentication-membership-and-tenancy.md).
