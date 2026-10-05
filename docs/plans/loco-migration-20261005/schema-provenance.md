# Persistence baseline provenance

P1 pins the preserved schema input; it does not adopt or restore it.

The source is retained PR #2419 at commit
`6d607c71c393a79cd259e39f13d48f1977fba1e2`, protected locally by
`refs/loco-migration/input-pr-2419`. Its `db/schema.sql` is copied byte for byte
to `migration/baseline.sql`, including its original comments and COPY metadata.
SHA-256: `f5043a97e60cea912a2f36dec79ee99c2d19e5a950916d1508f76d3e0b87966b`.

The same commit's `db/verify_schema.sql` has SHA-256
`6c4d1cf0783fcdaacb1e49a2abe05ba5b51602c811260cc7fe4c67a9f3dda269`.
It remains a retained input rather than a claim of complete catalog coverage.

All 177 source migration files under `db/migrate` match the current
`rails/db/migrate` files byte for byte. Versions span `20250623132120` through
`20261002174039`. This source reconciliation is distinct from a live database
catalog comparison or a populated rollback rehearsal.

The baseline contains PostgreSQL COPY-from-stdin metadata and must remain a
PostgreSQL administrative restore artifact. P2 will provision it only into an
explicit empty owned PostgreSQL 18 fixture using maintained psql tooling, then
perform guarded SeaORM adoption. Populated adoption must not execute its DDL.
No dump parser, schema adoption, runtime psql dependency, live database access or
credential interoperability proof is introduced by P1.
