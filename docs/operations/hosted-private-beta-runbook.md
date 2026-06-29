# Hosted Private Beta Runbook

This runbook covers the hosted private beta target: one Rails application and one
PostgreSQL database serving multiple independent households. The beta must remain
closed until the hosted hardening audit is green.

## Go/No-Go

Before onboarding another household, verify:

- The web process connects with `DATABASE_ROLE=med_tracker_app`.
- Migration/setup processes connect with the owner-capable migration role.
- `task rubocop`, `task test`, and `task brakeman` pass on the release branch.
- The hosted hardening audit has no `NO-GO` rows.
- Invite-only registration is pinned by environment or platform-admin policy.
- Backup and Restore test evidence exists for the current deployment.
- Support access is only available through the audited Platform admin flow.

## Onboarding

1. Confirm the release branch has passed the go/no-go checklist.
2. Create or select the target household through the hosted onboarding flow.
3. Create the first owner by invitation only.
4. Require the owner to configure MFA/passkey before any household admin page is usable.
5. Confirm the household owner can sign in, view only their household dashboard, and invite members.
6. Confirm another household account cannot access the new household by slug, API id, direct record id, or attachment URL.

## Support access

1. Platform admin signs in with MFA/passkey.
2. Platform admin opens a support access session for one household and records a reason.
3. The app records actor account, target household, request id, IP, reason, start time, and expiry.
4. Support mode is visually and technically distinct from household membership.
5. Platform admin cannot browse health or medicine data outside an active support access session.
6. Support access end and expiry are audited.

## Export and purge

Hosted beta uses configurable retention with export + purge as the default deployer
policy unless an explicit retention hold exists.

1. Verify the requester is an active owner of the household or an authorized platform admin in support mode.
2. Generate household export from household-scoped queries only.
3. Audit export request, generation, download, and expiry without storing raw medication names or dose notes in the audit metadata.
4. Revoke household memberships, API sessions, API app tokens, push subscriptions, and native device tokens during offboarding.
5. If no retention hold exists, purge tenant-owned rows and tenant-owned attachments.
6. If a retention hold exists, archive the household, disable access, document the hold reason and review date, and keep data out of normal household flows.

## Restore test

1. Restore the latest backup into an isolated environment.
2. Run migrations using the owner-capable migration role.
3. Run the app using `DATABASE_ROLE=med_tracker_app`.
4. Verify RLS default-denies without tenant context.
5. Verify each restored household can only see its own people, medicines, schedules, notifications, audit rows, and attachments.
6. Record restore date, backup identifier, app image/tag, migration version, tester, and outcome.

## Incident response

1. Disable affected credentials or households.
2. Preserve audit rows, support access sessions, logs, and request ids.
3. Export a tenant-scoped evidence bundle for investigation.
4. Patch and verify in an isolated environment.
5. Re-run the go/no-go checklist before re-enabling affected access.
