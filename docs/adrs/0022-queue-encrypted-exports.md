# 22. Queue encrypted household exports

Encrypted export requests submit a durable job. Clients use the returned job
location to check status and fetch the encrypted bundle. The same account must
authenticate for each request, and download rechecks household access and the
scope used to generate the export.

The household export row and its PostgreSQL queue entry commit together. Queue
arguments contain identifiers only. The export row holds a passphrase encrypted
with AES-256-GCM under a dedicated operator-provided key. The authentication data
binds the secret to its job, household and requester. A successful job replaces
the secret with the encrypted result in the same transaction as the export
audit. Workers need the same key across restarts and replicas.

PostgreSQL advisory transaction locks provide a shared concurrency limit without
adding a persistent lease that can outlive a worker. The configured limit
defaults to one. SeaORM builds and parameterises the call to PostgreSQL's
`pg_try_advisory_xact_lock` because its row-locking APIs do not expose advisory
locks. Workers release the transaction before waiting for a free slot.

Job and download access expires 24 hours after submission. The expiry check
controls API access. It does not physically purge database rows or backups.
