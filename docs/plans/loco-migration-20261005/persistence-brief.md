# Persistence adoption tranche brief

Prepared while foundation acceptance is pending; this is not an implementation
dispatch or permission to use a live database.

Input: PR #2419, exact head `6d607c71c393a79cd259e39f13d48f1977fba1e2`,
preserved at `refs/loco-migration/input-pr-2419`. Examine its baseline SQL,
verification queries and provenance against the relocated Rails migration source.
Do not assume its fixtures or catalog comparison are independently runnable.

Nightingale remains the sole source/test writer. The verifier owns all disposable
PostgreSQL 18 resources and build/runtime checks. Bucky owns planning records,
Git integration and independent Devin review dispatch.

Required observable RED acceptance before implementation:

- The empty Loco ledger cannot reproduce the Rails migration catalog.
- A repeated adoption must not duplicate DDL or change the normalized catalog.
- Deliberately introduced catalog drift must fail with a useful diagnosis.
- Populated synthetic records retain IDs, foreign keys, person enums, capacity
  rules and stored password/token bytes through compatible additive changes.
- A runtime application role cannot create or alter schema; framework queue
  storage is provisioned beforehand by the migration owner.

Compare tables, columns, types, defaults, nullability, indexes, constraints,
extensions, functions, triggers, views, grants and RLS policies. Record named
differences and preserve Rails migration-ledger provenance. Verify both fresh
installation and adoption of the representative Rails schema, including Rails
boot and relevant journeys on the adopted disposable database. Keep rollback
compatible by allowing only additive compatible changes while Rails is retained.

All fixture credentials use `password`. No live credentials or token data enter
the rehearsal. Preserving synthetic stored bytes proves persistence only; it
does not establish authentication interoperability.

Authentication selection remains a separate gate in auth-feasibility.md. This
tranche does not authorize bespoke security protocols or an external identity
service. Dispatch follows foundation acceptance and updated source examination.
