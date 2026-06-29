# Hosted Multi-Tenant Security Hardening Audit

Date: 2026-06-29

This audit is the gate for hosting more than one independent household in one Rails
application and one PostgreSQL database. Existing household-tenancy work is treated
as foundation evidence, not as proof that the hosted private beta is safe.

## Beta Decision

**Current status: NO-GO**

The app already has household routes, household memberships, person access grants,
tenant context, runtime database roles, forced RLS coverage, and API credentials
bound to household memberships. The hosted beta remains blocked until the no-go
items below are closed, verified, and documented.

## Decisions

- Tenant boundary: household.
- Deployment target: shared Rails app and shared PostgreSQL database.
- Platform admin: in-app operator capability, separate from household membership.
- Owner promotion: platform-admin-only during hosted beta.
- MFA/passkey: required before household admin pages are usable; fresh verification
  remains required for sensitive actions.
- API writes: in scope after RLS/runtime-role/tenant-write gates are complete.
- Professional/carer access: household-only for this epic.
- Retention: configurable per deployer. Hosted beta defaults to export + purge
  unless a documented retention hold exists.

## Requirements Matrix

| Requirement | Current evidence | Gap / decision | Severity | Owner issue | Tests required | Beta status |
| --- | --- | --- | --- | --- | --- | --- |
| FR1 | `SchemaInventory`, household models, composite foreign keys, tenant route specs. | Complete inventory must be rechecked against every tenant-owned table and future health/export records. | High | HMT-001 | Static inventory and cross-household association specs. | NO-GO |
| FR2 | Many tenant tables have `household_id`; some are still nullable in `db/schema.rb`. | Backfill remaining nullable tenant rows, fail with diagnostics when ownership cannot be derived, then enforce `NOT NULL`. | Critical | HMT-001 | Migration specs and schema nullability specs. | NO-GO |
| FR3 | Forced RLS specs exist for household rows, membership bootstrap, and ActiveStorage attachments. | RLS policies still need proof that null household rows are never globally visible. | Critical | HMT-001 | Runtime role default-deny, cross-household read/write, null-policy inspection. | NO-GO |
| FR4 | `med_tracker_app` role exists with `NOBYPASSRLS`; production web defaults to runtime role. | Need deploy-time proof that web uses runtime role and migrations use owner role outside local Compose. | Critical | HMT-002 | Role privilege specs and deployment config checks. | NO-GO |
| FR5 | Web routes use `household_slug`; API credentials bind to membership; `TenantContext` sets DB context. | API write credentials and support-mode context must follow the same membership-derived rule. | Critical | HMT-003 | Web/API membership rejection and DB context specs. | NO-GO |
| FR6 | Architecture spec blocks direct tenant `.find(params)` patterns. | Expand coverage to API write endpoints and background jobs introduced by this epic. | High | HMT-003 | Request specs for cross-household show/update/destroy/write attempts. | NO-GO |
| FR7 | Admin user controller scopes users and dependents to current household in several paths. | Generic user updates still control membership role and dependent/person grants; split into dedicated services/controllers. | Critical | HMT-004 | Request/policy specs proving generic user update cannot mutate roles or grants. | NO-GO |
| FR8 | Household policies use membership roles instead of legacy user role predicates; platform admin/support records now exist; app settings are platform-admin-only. | Support-mode controller flow, MFA/passkey challenge, reason capture UI, and audit event contract still need completion. | Critical | HMT-005 | Platform-admin policy specs, household-manager denial specs, support-session request specs, and audit specs. | NO-GO |
| FR9 | Invite-only mode, token digests, expiration, resend rotation, and invitation grants exist. | Acceptance under runtime RLS and invited-email immutability need explicit hosted-mode proof. | High | HMT-006 | Runtime-role invitation accept, invalid/reused/cross-household specs. | NO-GO |
| FR10 | Rodauth MFA/passkeys exist; API token creation checks MFA-satisfied session. | Household admin pages need hard MFA/passkey gate and passkey UV for sensitive actions. | Critical | HMT-007 | Admin-page redirect/denial specs and sensitive-action challenge specs. | NO-GO |
| FR11 | API sessions/app tokens are membership-bound and compare permissions version. | API writes, idempotency keys, and permission-version bumping on role/grant changes must be completed. | Critical | HMT-008 | API login throttling, stale-token, idempotency replay, write parity specs. | NO-GO |
| FR12 | Notification preferences and device tokens exist; jobs carry household id; medication reminders and native push logs no longer include medication names or notification body text. | Full recipient/grant-aware review and private-by-default coverage for every notification path are not complete. | Critical | HMT-009 | Recipient, preference, content-redaction, and log-redaction specs. | NO-GO |
| FR13 | Reminder jobs wrap household work in `TenantContext`. | All tenant-owned jobs need inventory and sensitive payload/log review. | High | HMT-010 | Job inventory specs and tenant-context reload specs. | NO-GO |
| FR14 | PaperTrail and `SecurityAuditEvent` include household fields in the cutover docs. | Hosted support access, export/delete, role/grant changes, and redaction rules need a single audit contract. | High | HMT-011 | Audit event coverage and redaction specs. | NO-GO |
| FR15 | Account close exists and reports page exists. | Household/person export, configurable retention, offboarding, purge, notification/token revocation, and runbook are missing. | Critical | HMT-012 | Export auth, purge, retention-hold, and token/device revocation specs. | NO-GO |
| FR16 | Reports are scoped through `policy_scope(Person)`. | Health-event/PDF/report artifact lifecycle must be tenant-owned and audited. | High | HMT-013 | Health-event, PDF generation, expiring link, and audit specs. | NO-GO |
| FR17 | ActiveStorage attachments carry `household_id`; default direct uploads are disabled. | Attachment nullability and blob access routes need hosted-mode review for every tenant-owned attachment type. | High | HMT-014 | Avatar/report attachment RLS and route authorization specs. | NO-GO |
| FR18 | OTLP exporter is allowlisted; telemetry specs exist. | Notification/native logs and all new API/write/export/support paths need sensitive-field review. | High | HMT-015 | Log filter, span allowlist, and background payload redaction specs. | NO-GO |
| NFR1 | RSpec coverage exists for RLS, policy scopes, API auth, telemetry, and invitations. | Add coverage for the new hosted hardening matrix, platform admin, API writes, export/offboarding, and retention. | High | HMT-016 | Full focused specs listed by child issue. | NO-GO |
| NFR2 | Local household migrator supports dry run and apply. | Hosted migration strategy needs idempotent backfills, diagnostics, and runtime-role compatibility documentation. | Critical | HMT-017 | Migration dry-run/data diagnostics and role compatibility specs. | NO-GO |
| NFR3 | Local migrator preserves existing single-household access. | API/session/device/export behavior needs explicit migration or invalidation decision. | High | HMT-018 | Backward-compatibility migration and token invalidation specs. | NO-GO |
| NFR4 | Production-style Compose and some Kubernetes runbooks exist. | Go/no-go checklist, onboarding, offboarding/export/delete, restore test, and support-access runbooks are incomplete. | Critical | HMT-019 | Documentation specs and operational checklist review. | NO-GO |

## Child Issue Backlog

| Issue | Title | Exit criteria |
| --- | --- | --- |
| HMT-001 | Enforce strict tenant ownership and RLS null denial | Tenant-owned tables have non-null household ownership and RLS policies no longer expose null household rows. |
| HMT-002 | Prove deployment runtime role separation | Production runtime role is `med_tracker_app`, migrator role is owner-capable, and app role cannot migrate or bypass RLS. |
| HMT-003 | Complete tenant context and controller/job scoping audit | Web, API, and jobs derive tenant context from active membership and reject cross-household access. |
| HMT-004 | Split user, membership, and person-access management | Generic user update cannot mutate membership role, owner status, or dependent/person grants. |
| HMT-005 | Add platform admin and support access | Platform admin is separate from household admin; support access requires reason, MFA/passkey, explicit session, and audit. |
| HMT-006 | Harden invitation/signup under runtime RLS | Invite acceptance is safe under runtime role, email is fixed to the invitation, and reused tokens fail. |
| HMT-007 | Enforce MFA/passkey for admin access and sensitive actions | Household admin pages are blocked without MFA/passkey; sensitive actions require fresh verification. |
| HMT-008 | Add tenant-safe API writes | API writes use shared domain services, idempotency, membership-bound credentials, and audit-safe contracts. |
| HMT-009 | Make notifications private by default | Notification recipients are grant-aware and content/logs avoid medication or health details unless explicitly enabled. |
| HMT-010 | Carry tenant context through every job | Tenant-owned jobs accept or derive household safely and reload rows inside `TenantContext`. |
| HMT-011 | Define hosted audit contract | Security actions are audited with tenant/actor/request context and without raw health content. |
| HMT-012 | Implement export, offboarding, retention, and purge | Deployer retention is configurable; hosted beta can export then purge with audited retention holds. |
| HMT-013 | Tenant-safe health events and generated reports | Health events and generated artifacts are household-owned, authorized, expiring, and audited. |
| HMT-014 | Finish attachment isolation | Tenant-owned attachments are non-null household-owned and blob routes do not leak cross-tenant artifacts. |
| HMT-015 | Finish telemetry and logging redaction | Logs/spans/background traces avoid email, token, medication, dose, diagnosis, and free-text health content. |
| HMT-016 | Complete hosted hardening test inventory | Every FR/NFR has a focused automated test or a documented manual operational check. |
| HMT-017 | Document safe migration strategy | Backfills are idempotent and fail with actionable diagnostics when ownership cannot be assigned. |
| HMT-018 | Preserve single-household compatibility | Existing self-hosted installs migrate into one household or have explicit invalidation behavior. |
| HMT-019 | Complete hosted operations runbooks | Go/no-go, onboarding, offboarding/export/delete, restore, and support-access procedures are documented. |

## Go/No-Go Checklist

- [ ] Runtime app DB role is `NOBYPASSRLS` and not owner.
- [ ] Tenant-owned tables enforce non-null household ownership.
- [ ] RLS default-denies without tenant context and never exposes null household rows.
- [ ] Invite-only registration is pinned for hosted beta.
- [ ] Invitation acceptance works under runtime RLS.
- [ ] Generic admin user update cannot change roles or person grants.
- [ ] Household admins cannot update platform/global settings.
- [ ] Platform admin support access is explicit, MFA/passkey protected, reasoned, and audited.
- [ ] API login, tokens, sessions, and writes are tenant-safe.
- [ ] Household admin pages require MFA/passkey.
- [ ] Notifications are private-by-default and person-access aware.
- [ ] Export, retention, offboarding, and purge exist.
- [ ] Backup restore has been tested.
- [ ] Audit, logs, and telemetry are redacted for hosted health data.
