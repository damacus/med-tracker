# Audit Evidence Operations

## Production gate

Do not enable WORM delivery until the records manager/DPO has approved the retention schedule, the bucket is versioned with Object Lock at creation time, encryption and expected owner are confirmed, governance mode has been exercised, and the exporter/verifier credentials have been proven unable to delete objects, shorten retention, bypass governance, read clinical tables, or modify ledger history. Record a representative concurrent-write load test and a ten-year database/object-store capacity estimate using measured event volume and payload sizes before approval.

Compliance mode requires a separate records-governance change approval and `AUDIT_WORM_COMPLIANCE_APPROVED=true`.

The capacity record must include peak audited writes per second, p95 insert latency, daily event count, average and p95 ledger/object bytes, index and backup overhead, replication, checkpoint/export overhead, forecast assumptions, safety margin, estimated cost, and the date when the estimate must be reviewed. Do not infer production capacity from the six-thread serialization regression spec.

## Schedule

- Run `SCOPE=database FORMAT=json task audit:verify` as a full daily check.
- Run `SCOPE=worm FORMAT=json task audit:verify` on a rotating object sample daily and all objects at least monthly.
- Run `task audit:monitor` at least once per minute.
- Alert when the oldest pending delivery reaches five minutes; page at one hour.
- Keep command output and alerts free of household, person, medication, event, and source identifiers.

## Baseline

Immediately after migration, sign every `legacy-baseline` checkpoint and export the native evidence and manifest to Object Lock. Record the manifest checksum, object version, public key ID, deployment version, migration time, and external change reference. State that integrity is proven from the baseline onward only.

## Signing-key rotation

1. Generate Ed25519 key material in the approved key-management system.
2. Configure the new key ID/private key only in the exporter or one-off verifier process.
3. Produce and verify a checkpoint with the new key.
4. Retire the old private key from active use.
5. Retain every old public key and signed checkpoint indefinitely while dependent evidence exists.
6. Record the rotation in the external change system and export the resulting checkpoint.

## Backlog response

At five minutes, check exporter health, credentials, bucket owner/versioning/Object Lock/encryption, and delivery error codes. At one hour, page incident response. Do not discard rows, reset attempts, weaken retention, overwrite objects, or grant the exporter broader database access. Configuration/integrity failures require human resolution; temporary failures remain pending with bounded retry.

## Integrity incident

1. Stop affected exporter/verifier credentials without stopping local ledger writes.
2. Preserve database snapshots, WORM versions, public keys, manifests, app/deployment logs, and request/trace IDs.
3. Run combined JSON verification and store the unchanged output in the incident record.
4. Identify the first invalid chain/sequence without exporting unrelated household data.
5. Treat checksum, duplicate-version, retention, missing-object, source mismatch, and tail-truncation findings as evidence incidents.
6. Do not repair or re-chain history in place. Recovery creates a new documented epoch/checkpoint after evidence preservation and approval.

## Restore and disaster recovery

After restoring PostgreSQL, run full database verification before enabling traffic. Compare every restored chain head and checkpoint with the latest signed WORM evidence. A backup that is internally valid but behind WORM is a restore-divergence incident; replay through an approved recovery process rather than deleting external evidence.

Record backup identifier, database/app version, restore point, verifier version, manifest/checkpoint IDs, result, operator, and incident/change reference. Test restore and divergence handling at least quarterly.

## Legal hold and disposal

Follow [the retention policy](../compliance/audit-retention-policy.md). Holds are controlled in the external records system and may extend but never shorten WORM retention. There is no automatic audit-evidence disposal. Database-owner actions require an external incident/change record.
