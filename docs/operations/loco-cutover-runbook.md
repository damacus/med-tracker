# Loco cutover and saved-state rollback

This procedure applies only after the complete replacement has passed its release gates and the owner has separately authorised a live cutover. The rehearsal uses synthetic data and an owned PostgreSQL 18 project. It does not connect to production.

## Rehearsal gate

Run `task release:rehearse`. The Task builds the current Rails test image through `task rails:test:build CONTRACT_PROJECT=mtloco-rollback-image`, records the immutable image ID of `mtloco-rollback-image-web-test`, and passes that ID to the Rails journey. It creates an isolated PostgreSQL 18 project, provisions the Rails baseline, and seeds the existing synthetic household and medication fixtures. The script checks all four PostgreSQL client binaries and the server are version 18, rejects a non-loopback endpoint and an unowned project name, and checks that no Rails or Loco database client remains connected. It saves a full custom-format pre-Loco dump and records a digest of the complete plain dump with PostgreSQL's fixed comparison-only restrict key.

The rehearsal adopts the schema, runs the Loco dose journey on that same populated database, and requires the Loco process to exit. It checks for connected writers again, drops and recreates only its owned `medtracker_reference` database, and restores the independent saved dump with its original object ownership, grants and row security policies. Database roles remain in the same isolated PostgreSQL project throughout the rehearsal; a separate global-object digest checks that their definitions and memberships did not change. Complete database and global-object digests must match the pre-Loco state before Rails runs. The Rails journey signs in with the saved credential, reads the household dashboard, and records a dose. The owned project and temporary dump are removed after success or failure. The dump is never a production backup.

The Loco journey must show a single dose and audit after duplicate replay, stock reduced once, and cross-household denial. Broader release evidence must cover native sign-in and dose use, fresh offline replay, worker restart, session and token invalidation, passkey replacement and recovery, and underlying file access. A missing journey or Task fails the rehearsal. A passing orchestration test alone does not satisfy the release gate.

## Before a live cutover

Record the approved image digest for each production architecture, server and worker startup results, the exact schema revision, current backups, a restoration target, and the chosen cutover time. Confirm complete capability acceptance, both scratch runtime checks, hosted CI, and `task release:rehearse` on the reviewed commit. Keep the reviewable result and approval separate from execution.

Tell affected users that old sessions and tokens will expire. Historical passwords and MFA enrolments may need secure reset or new enrolment. Unsupported passkeys, including PS256, require replacement; supported keys on mixed accounts remain usable. Do not bypass MFA during recovery. Old download links, system export formats, offline queues, and push subscriptions may need renewal. Existing clinical records, identifiers, audit history, and underlying files must remain intact through adoption.

## Cut over after approval

1. Stop Rails web servers, separately deployed workers and schedulers. Verify that no Rails writer retains a database session and that pending jobs are within the accepted loss boundary.
2. Freeze the rollback replica so it stops following, or take and verify an independent pre-Loco full database dump. Preserve that state and the matching Rails image and credentials outside the Loco write path. Record its immutable identity and a restore test.
3. Check the deployed Loco image digest and required credentials, host settings, connection limits, storage, TLS, PDF fonts, timezone assets, and server and worker health. Adopt the schema with the approved migration Task and verify ledger and data checks before allowing traffic.
4. Start Loco server, workers and schedules only after Rails writers are stopped. Prove exclusive writer ownership. Check fresh sign-in and recovery, household isolation, clinical dose and audit, native API, browser, notifications, and worker restart. Record a go/no-go decision against the approved release evidence.

Do not use the synthetic rehearsal commands or its disposable credentials against a live database.

## Roll back after approval

1. Stop Loco web servers, workers and schedulers. Verify that no Loco writer retains a session before restoring data.
2. Restore the independent pre-Loco dump to a separate Rails target, or promote the replica that was frozen before Loco wrote. Check the saved snapshot identity, schema, credential state, record IDs and audit history. Never restore over an active Loco writer.
3. Start Rails web servers, workers and schedulers against that restored target. Prove sign-in, household reads, dose writes and audit recording on the restored state before routing traffic back.
4. Record the rollback time and the accepted data gap. New data created during the Loco period may be discarded or ignored. Lossless rollback and Rails compatibility with Loco-created records are outside the approved contract.

No production shutdown, restore, routing change or deployment is authorised by this runbook.
