# Persistence adoption tranche brief

Prepared while foundation acceptance is pending; this is not an implementation
dispatch or permission to use a live database.

Input: PR #2419, exact head `6d607c71c393a79cd259e39f13d48f1977fba1e2`,
preserved at `refs/loco-migration/input-pr-2419`. Examine its baseline SQL,
verification queries and provenance against the relocated Rails migration source.
Do not assume its fixtures or catalog comparison are independently runnable.

Concrete source inspection during foundation verification identifies the retained
paths as `db/schema.sql` and `db/verify_schema.sql`. The baseline includes PostgreSQL
`COPY ... FROM stdin` and `\.` migration-metadata sections; it is a psql restore
artifact, not directly executable as one SeaORM raw SQL string. Preserve its bytes
and use PostgreSQL's maintained restore tooling for the reference rehearsal.
Fresh installation uses explicit administrative PostgreSQL 18 psql provisioning
of an empty owned database, followed by guarded standard SeaORM adoption. Preserve
the dump byte-for-byte, including COPY metadata; the scratch application needs no
psql and the runtime migration never parses or executes the dump. Populated
adoption skips provisioning. Begin an outer database transaction, verify the
catalog before invoking `Migrator::up`, verify again, then commit. Preflight must
run on repeated adoption too: the standard migrator skips an applied baseline,
and its ledger installation must not precede a failed drift check. Pinned SeaORM
2.0.4 accepts a borrowed `DatabaseTransaction`; PostgreSQL migration execution
uses transactional migration bodies and ledger writes. Tests must prove these
boundaries with the actual pinned code. Do not invent a regex SQL/dump parser or silently omit the
Rails metadata rows. Read-only reconciliation confirms the artifact's COPY
ledger, retained migration filenames and current `rails/db/migrate` agree on
all 177 versions and migration byte hashes, spanning `20250623132120` through
`20261002174039`. The captured schema SHA-256 is
`f5043a97e60cea912a2f36dec79ee99c2d19e5a950916d1508f76d3e0b87966b`;
verification SQL SHA-256 is
`6c4d1cf0783fcdaacb1e49a2abe05ba5b51602c811260cc7fe4c67a9f3dda269`.
The retained verification SQL covers selected policies and audit objects; full
catalog parity needs broader acceptance. The dump references owner/grantee roles
and test-environment Rails metadata. Explicit administrative provisioning and
environment metadata handling must be tested separately from populated adoption.

The installed Loco 1.2.0 PostgreSQL queue initializer checks table existence and
the `priority` column before issuing CREATE/ALTER. P3 should provision the complete
`pg_loco_queue` table, including `priority`, and grant the runtime's required
access beforehand. Prove least-privilege startup with the actual pinned code;
do not grant schema-owner privileges to avoid an incomplete provisioning step.

Nightingale remains the sole source/test writer. The verifier owns all disposable
PostgreSQL 18 resources and build/runtime checks. Bucky owns planning records,
Git integration and independent Devin review dispatch.

Required observable RED acceptance before implementation:

- The current empty migrator cannot record the supported adoption row and accepts
  deliberate drift in an administratively restored schema. Catalog parity after
  restoration alone cannot prove that adoption works.
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

## P2 preparation decisions

Read-only inspection confirmed five required baseline roles and the `citext`,
`pg_trgm` and `pgcrypto` extensions. Restore into a uniquely owned empty fixture
with PostgreSQL 18 psql, startup files disabled, stop-on-error and one transaction.
Reference and candidate must share database locale and extension versions.
Preserve the dump's test-environment Rails metadata in test rehearsals. Populated
adoption never rewrites that metadata; fresh non-test provisioning must make its
environment handling explicit rather than silently changing the dump.

The migration owner owns `public.seaql_migrations`. Runtime roles receive no
write access to migration history. The baseline owner's default grants would
otherwise grant runtime writes: revoke those grants on this one ledger inside
the same adoption transaction and verify the exact resulting ACL. Leave all
application-table default privileges intact. Reject a pre-existing empty ledger,
malformed ledger or unknown migration rows; support only an absent ledger on
first adoption or the exact supported row on repeat adoption. Preserve the
recorded application timestamp on repeats.

Use PostgreSQL's transaction advisory locking for cooperating administrative
adoption calls, then preflight, standard `Migrator::up` on the borrowed transaction,
postflight and commit. This lock does not claim to prevent unrelated administrator
DDL. Require controlled administrative schema access during installation; prove
repeat and concurrent guarded-adoption behaviour on owned fixtures.

Compare full catalog definitions, owners, ACLs and RLS flags/expressions, including
unexpected objects. The retained verification SQL remains a partial smoke check.
Capture the actual pinned SeaORM-generated ledger catalog before recording its
exact allowed differences. Interpret repeated adoption's zero DDL changes as no
effective catalog change; standard `CREATE TABLE IF NOT EXISTS` may still execute.
Fresh installation, populated preservation, failure rollback and drift rejection
remain separate behavioural checks. No schema has been adopted by this preparation.
