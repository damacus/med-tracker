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
| FR1 | `SchemaInventory` classifies every primary application table exactly once; household-owned entries are physical tables and expose `household_id`; `versions` is classified as global pending the FR14 audit-contract work. | Closed. | High | HMT-001 | `spec/lib/schema_inventory_spec.rb`. | GO |
| FR2 | `20260624091000_enforce_strict_household_tenant_boundary.rb` backfills derivable tenant rows, creates a single legacy household only for unambiguous single-household roots, fails ambiguous orphan roots, and enforces `household_id NOT NULL` on every household-owned table in `db/schema.rb`. | Closed. | Critical | HMT-001 | `spec/lib/schema_inventory_spec.rb`. | GO |
| FR3 | PostgreSQL RLS is enabled and forced for every household-owned table; live policy specs prove no public RLS policy exposes `household_id IS NULL`, runtime default-denies without tenant context, and cross-household reads are isolated. | Closed. | Critical | HMT-001 | `spec/models/household_row_level_security_spec.rb`. | GO |
| FR4 | Runtime role specs prove `med_tracker_app` is `NOBYPASSRLS`, not superuser, cannot create in `public`, and is not a member of `med_tracker_owner`; Compose specs prove prod migrations use the owner-capable role while web runtime uses the restricted runtime role. | Closed. | Critical | HMT-002 | `spec/models/household_row_level_security_spec.rb`, `spec/config/yaml_compose_spec.rb`, `spec/config/database_role_config_spec.rb`. | GO |
| FR5 | Web routes use `household_slug`; API credentials bind to membership; `TenantContext` sets DB context. | API write credentials and support-mode context must follow the same membership-derived rule. | Critical | HMT-003 | Web/API membership rejection and DB context specs. | NO-GO |
| FR6 | Architecture spec blocks direct tenant `.find(params)` patterns. | Expand coverage to API write endpoints and background jobs introduced by this epic. | High | HMT-003 | Request specs for cross-household show/update/destroy/write attempts. | NO-GO |
| FR7 | `Admin::UsersController#update` now permits only email/person profile fields; `Admin::MembershipRolesController` and `Admin::MembershipRoleUpdater` handle non-owner role changes with `SecurityAuditEvent` records; edit UI posts role changes to the dedicated endpoint. | Person/dependent access still needs final dedicated-service reconciliation across every admin path; owner promotion remains rejected here until the platform-admin owner-promotion flow is implemented. | Critical | HMT-004 | `spec/requests/admin/user_mutation_boundary_spec.rb`, `spec/requests/admin_create_update_turbo_spec.rb`, and policy specs for remaining role/grant paths. | NO-GO |
| FR8 | Global `platform` routes, `Platform::SettingsController`, `Platform::SupportAccessSessionsController`, `SupportAccessSessionPolicy`, and support-mode authorization context exist; support sessions capture actor, target household, reason, MFA proof time, request id, IP, expiry/end, and redacted audit events without granting household membership. | Support-mode operator UI, expiry/timeout operational checks, and broader platform-admin runbook coverage still need completion. | Critical | HMT-005 | `spec/requests/platform/settings_spec.rb`, `spec/requests/platform/support_access_sessions_spec.rb`, platform-admin policy specs, and support-mode audit/runbook checks. | NO-GO |
| FR9 | Invitation acceptance pins the account login to the invitation email in `rodauth_main`; request specs prove submitted email is ignored and accepted/expired tokens do not create accounts; model specs prove invitation audit versions omit raw token material. | Runtime-role invitation acceptance under production `med_tracker_app` RLS and cross-household invitation visibility still need explicit hosted-mode proof. | High | HMT-006 | `spec/requests/invitations_spec.rb`, `spec/models/household_invitation_spec.rb`, runtime-role invitation acceptance, invalid/reused, and cross-household specs. | NO-GO |
| FR10 | `HostedPrivilegedActionMfa` gates hosted household admin controllers behind `HOSTED_ADMIN_MFA_REQUIRED`; request specs prove admin pages redirect without local MFA/passkey or upstream OIDC MFA evidence and allow MFA/OIDC-verified sessions. | Fresh sensitive-action challenge coverage and passkey user-verification requirements still need to be applied to every high-risk mutation. | Critical | HMT-007 | `spec/requests/admin_mfa_gate_spec.rb`, touched admin request specs, and sensitive-action fresh-challenge specs. | NO-GO |
| FR11 | API sessions/app tokens are membership-bound and compare permissions version. | API writes, idempotency keys, and permission-version bumping on role/grant changes must be completed. | Critical | HMT-008 | API login throttling, stale-token, idempotency replay, write parity specs. | NO-GO |
| FR12 | Notification preferences and device tokens exist; jobs carry household id; medication reminders and native push logs no longer include medication names or notification body text. | Full recipient/grant-aware review and private-by-default coverage for every notification path are not complete. | Critical | HMT-009 | Recipient, preference, content-redaction, and log-redaction specs. | NO-GO |
| FR13 | Reminder jobs wrap household work in `TenantContext`. | All tenant-owned jobs need inventory and sensitive payload/log review. | High | HMT-010 | Job inventory specs and tenant-context reload specs. | NO-GO |
| FR14 | PaperTrail and `SecurityAuditEvent` include household fields in the cutover docs. | Hosted support access, export/delete, role/grant changes, and redaction rules need a single audit contract. | High | HMT-011 | Audit event coverage and redaction specs. | NO-GO |
| FR15 | Account close exists and reports page exists. | Household/person export, configurable retention, offboarding, purge, notification/token revocation, and runbook are missing. | Critical | HMT-012 | Export auth, purge, retention-hold, and token/device revocation specs. | NO-GO |
| FR16 | Reports are scoped through `policy_scope(Person)`. | Health-event/PDF/report artifact lifecycle must be tenant-owned and audited. | High | HMT-013 | Health-event, PDF generation, expiring link, and audit specs. | NO-GO |
| FR17 | ActiveStorage attachments carry `household_id`; default direct uploads are disabled. | Attachment nullability and blob access routes need hosted-mode review for every tenant-owned attachment type. | High | HMT-014 | Avatar/report attachment RLS and route authorization specs. | NO-GO |
| FR18 | OTLP exporter is allowlisted; telemetry specs exist. | Notification/native logs and all new API/write/export/support paths need sensitive-field review. | High | HMT-015 | Log filter, span allowlist, and background payload redaction specs. | NO-GO |
| NFR1 | RSpec coverage exists for RLS, policy scopes, API auth, telemetry, invitations, hosted MFA gate, platform support sessions, membership-role mutation boundary, and this hosted hardening matrix. | Add remaining coverage for API writes, export/offboarding, retention, full attachment isolation, and operational checks. | High | HMT-016 | Full focused specs listed by child issue plus documentation matrix coverage in `spec/lib/schema_inventory_hosted_multitenant_hardening_documentation_spec.rb`. | NO-GO |
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

- [x] Runtime app DB role is `NOBYPASSRLS` and not owner.
- [x] Tenant-owned tables enforce non-null household ownership.
- [x] RLS default-denies without tenant context and never exposes null household rows.
- [ ] Invite-only registration is pinned for hosted beta.
- [ ] Invitation acceptance works under runtime RLS. Partial evidence: email pinning and invalid/reused token request specs pass.
- [ ] Generic admin user update cannot change roles or person grants. Partial evidence: role mutation boundary specs pass; remaining person-grant reconciliation is open.
- [ ] Household admins cannot update platform/global settings. Partial evidence: global platform settings request specs pass.
- [ ] Platform admin support access is explicit, MFA/passkey protected, reasoned, and audited. Partial evidence: support session request specs pass; operational checks remain open.
- [ ] API login, tokens, sessions, and writes are tenant-safe.
- [ ] Household admin pages require MFA/passkey. Partial evidence: hosted MFA gate request specs pass; fresh sensitive-action checks remain open.
- [ ] Notifications are private-by-default and person-access aware.
- [ ] Export, retention, offboarding, and purge exist.
- [ ] Backup restore has been tested.
- [ ] Audit, logs, and telemetry are redacted for hosted health data.
