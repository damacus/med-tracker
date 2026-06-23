# Household Tenancy Cutover Mapping

Issue: GitHub #1378

This branch introduces household tenancy as the new authorization boundary. Person access for household routes is explicit through `PersonAccessGrant`; location membership remains metadata for inventory grouping and legacy UI behavior. The local migrator still tolerates pre-cutover `users.role` and `accounts.subscription_plan` columns when they are present, but the final schema drops both.

## Acceptance Mapping

| Requirement | Branch evidence |
| --- | --- |
| Household tenancy foundation | `Household`, `HouseholdMembership`, `PersonAccessGrant`, household invitations, invitation grants, and `SecurityAuditEvent` models plus migrations `20260622000100` through `20260622001200`. |
| Subscription plan moves to household | `Household#subscription_plan` is introduced and `PaidFeature` resolves entitlement through the current or first active household; the local backfill migrator reads pre-cutover `accounts.subscription_plan` only when that column exists, and final migration `20260622001200` drops it. |
| Tenant-owned domain rows carry `household_id` | Foundation migration adds `household_id`; `SchemaInventory` and tenant integrity specs verify household-owned tables and composite tenant constraints. |
| Forced PostgreSQL RLS | `med_tracker.current_*()` helper functions, runtime roles, forced RLS migration, role convergence migration, and RLS specs verify default-deny, household isolation, app role restrictions, object ownership, and account-scoped membership bootstrap. |
| Local one-household migration path | `task households:migrate_local OWNER_EMAIL=... HOUSEHOLD_NAME=... DRY_RUN=1|APPLY=1` delegates to `Households::LocalMigrator`; service and rake specs cover dry run, idempotent apply, and owner validation. |
| Web/API household context | Web routes under `/households/:household_slug/...` and API data routes only under `/api/v1/households/:household_id/...`; controller and architecture specs verify `TenantContext`, grant-scoped results, and no unscoped API data resources. |
| API session binding | `ApiSession` binds to `household_membership_id` and `permissions_version`; login/refresh specs cover household binding, revocation, rotation, and stale permission invalidation. |
| Explicit person grants | Household Pundit paths use `AuthorizationContext` plus `PersonAccessGrant` levels `view`, `record`, and `manage`; specs cover owner role not exposing capable adults and revoked/expired grants stopping access. |
| User role cleanup | `User#role` enum/predicates are removed from app authorization; membership roles and `PersonAccessGrant` drive access, and final migration `20260622001200` drops the legacy `users.role` column. |
| Jobs and notifications partitioned | Reminder jobs now carry `household_id`; scheduler iterates household-scoped notification preferences; job specs and architecture specs guard the job signature. |
| Offline snapshots partitioned | Offline shell endpoints switch to household routes in household context, IndexedDB snapshots/queued takes/failed takes are keyed by household and membership, and system/request specs cover sync. |
| Attachments partitioned | `active_storage_attachments` carries `household_id`, copies it from the attached record, and has forced RLS; avatar and RLS specs cover household isolation. |
| Audit rows partitioned | PaperTrail `versions` and `security_audit_events` carry household and actor membership identity; auth token, AI medication, external lookup, and request specs cover partitioned writes. |
| Cache guardrails | Tenant-owned application code is blocked from raw `Rails.cache` use; only global external catalogue clients are allowlisted by architecture spec. |
| Turbo target partitioning | `TenantDomTargetsHelper` prefixes tenant record DOM IDs and Turbo targets with household identity in household context; people, medication, location, schedule, person-medication, and dashboard timeline targets use it, with focused helper/component/architecture specs. |
| Telemetry PHI reduction | OTLP export is allowlisted and `MedicationTake` no longer emits medication/person payload identifiers; telemetry specs verify the old mutating sanitizer is not registered and PHI attributes are dropped. |
| Database role wiring | `DATABASE_ROLE` maps to Rails PostgreSQL session variables for primary tenant connections, Compose bootstraps `med_tracker_owner`/`med_tracker_app`, production web defaults to `med_tracker_app`, and local dev/test can opt into runtime role enforcement once legacy setup code is fully household-contextual. |

## Remaining Cutover Work

Known follow-up audit items that may be split after the breaking cutover:

- Audit any remaining non-stream tenant DOM IDs that are only form-control selectors today, then decide whether they also need household prefixes.
- Enable `DATABASE_ROLE=med_tracker_app` by default in dev/test containers after remaining legacy specs and setup paths create tenant rows inside `TenantContext`.
