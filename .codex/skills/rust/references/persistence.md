# Persistence

Start with the existing entities and the relevant write/read module. Keep
SeaORM persistence and targeted SQL at their established boundaries; a new
data-access abstraction needs a concrete problem to justify it.

Check these project-specific invariants when the change touches them:

- Restricted database role and transaction-local tenant context remain on the
  connection performing the query. Check pooled-connection reuse for leakage.
- Reauthorisation and existing locking remain inside the clinical transaction.
  A permission check before a concurrent membership change is insufficient.
- Mutation, audit and sync events retain their required atomicity. Exercise
  failure paths, not just the final successful row.
- Idempotent replay retains payload-conflict detection and stock/dose effects
  occur once. Test retries and competing requests where these paths change.
- Dose and stock values preserve Decimal precision, schema bounds and unit
  semantics across parsing, calculation and serialisation.
- Schedule eligibility uses the existing clock and display-timezone rules;
  cover day boundaries and daylight-saving transitions when affected.

Apply visibility before pagination, aggregation or projection. Fetch related
rows in bounded batches at the request boundary. Do not hide unbounded loading
behind an apparently small response page.

Keep PostgreSQL and shared Rails schema changes explicit. Schema fixtures may
omit migration-created triggers or views; verify those against a migrated
database when the change depends on them.
