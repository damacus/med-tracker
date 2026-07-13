# Hosted Beta Security Gates Implementation Plan

Resolve GitHub issues #1618, #1619, #1620, and #1621 in order. Each task is implemented by a fresh subagent, independently reviewed, and committed before the next task starts.

## Global Constraints

- Follow `AGENTS.md`, including Serena-first navigation, the Ruby skill, Red-Green-Refactor, Fish syntax, and repository `task` wrappers.
- Do not implement DDD redesign work or alter medication dose-history semantics.
- Treat household isolation, authorization, credential revocation, audit integrity, and PHI-free metadata as security boundaries.
- Preserve existing public web and API contracts unless an issue explicitly requires a new contract; update OpenAPI documentation for changed API surfaces.
- Use transactions and bang persistence for related writes. Make retryable operational workflows idempotent and safe after partial failure.
- Never put invitation tokens, medication names, dose notes, attachment contents, or other health data in audit metadata, logs, evidence manifests, or filenames.
- Add migrations and update `db/schema.rb` only through the repository migration path; PostgreSQL remains version 18 and forced RLS must continue to cover every tenant-owned table.
- Tests must prove observable behavior at web/API/service/database boundaries, including cross-household denial. Every production change begins with a failing focused spec and the report records RED and GREEN evidence.
- Run focused specs during each task. Before publication run `task rubocop`, `task test`, and `task brakeman`; never push a failing branch.
- Commit only task-scoped changes with Conventional Commit messages. Do not revert or rewrite work from earlier tasks.
- For #1621, never fabricate restore evidence. A rehearsal is complete only if a real current database and attachment backup is restored into an isolated production-like environment and the required durable evidence is recorded. If required backup/environment credentials are unavailable, complete safe executable automation and report the exact external blocker without closing the issue.

## Task 1: Resolve #1618 — invalidate hosted credentials when access changes

### Outcome

Every effective membership or person-access change goes through one transactional access-change boundary, advances `permissions_version`, invalidates stale hosted credentials, preserves owner governance, and emits a PHI-free audit record. A platform administrator can promote a household owner only after fresh privileged-action MFA.

### Primary implementation surfaces

- Introduce a shared access-change service under `app/services/households/` (or the nearest established service namespace) for membership role/status/person changes and person-access grant create/update/revoke changes.
- Route `app/services/admin/membership_role_updater.rb`, `app/controllers/api/v1/admin/memberships_controller.rb`, `app/controllers/api/v1/admin/person_access_grants_controller.rb`, and every other discovered mutation path through that service.
- Add a platform-admin-only owner promotion endpoint and Phlex/RubyUI surface under the existing `Platform` namespace. Protect it with the existing hosted privileged-action MFA gate and the household owner-governance rules.
- Reuse `ApiSession`, `ApiAppToken`, and `OauthGrant` permission-version validation; make the access-change transaction advance the affected membership version exactly when effective access changes.
- Define and test the pre-hosted credential compatibility decision. Credentials without a valid current membership/version binding must fail closed.
- Emit success and rejected-attempt audit events with actor account/membership, target membership, old/new access state, household, request context, and outcome; exclude health data.

### Required RED/GREEN coverage

- Service specs for role, status, person reassignment, grant create/change/revoke, no-op changes, rollback, and owner-governance invariants.
- Web request specs proving the existing role path invalidates stale API sessions, app tokens, and OAuth grants.
- API request specs proving membership and grant mutations invalidate the same credentials and cannot target another household.
- Platform request/policy specs for owner promotion: platform admin only, fresh MFA required, stale/missing MFA rejected, target household isolation, last-owner invariants, and audit outcomes.
- Migration/model specs for legacy or missing permission-version bindings failing closed, if schema/data migration is required.
- System/browser coverage and desktop/mobile screenshots if the owner-promotion UI is visible.

### Verification

- Run each changed focused spec through `task test TEST_FILE=...`.
- Run `task rubocop` for task-scoped quality before committing.
- Commit with a Conventional Commit message that references the credential/access-change behavior.

## Task 2: Resolve #1619 — close invitation and support-session lifecycle gaps

### Outcome

Invitation acceptance is proven under the forced-RLS application role and cannot cross household boundaries. Support access expires immediately in authorization and natural expiry is recorded exactly once by an idempotent operational path.

### Primary implementation surfaces

- Extend invitation model/request/API coverage around `HouseholdInvitation`, `HouseholdInvitationGrant`, `InvitationsController`, the API admin invitation controller, and Rodauth acceptance using `spec/support/database_runtime_role_setup.rb`.
- Ensure every invitation read/accept/revoke/reuse path is household-scoped under `DATABASE_ROLE=med_tracker_app` with forced RLS. Keep the accepted account email pinned to the invitation.
- Add an idempotent support-session expiry service and executable job/task using the existing `SupportAccessSession` and audit event patterns. Natural expiry must create one PHI-free `support_access_session.expired` audit event even under retries or concurrent processing.
- Preserve immediate `active?`/scope denial at expiry. Explicit end and natural expiry must remain distinguishable and must not duplicate audit events.
- Update `docs/operations/hosted-private-beta-runbook.md` with the executable operator check and expected sanitized audit evidence.

### Required RED/GREEN coverage

- Runtime-role request/integration specs proving invitation acceptance succeeds as `med_tracker_app` and forced RLS remains active.
- Cross-household specs covering invitation and grant read, accept, revoke, and token reuse denial; raw tokens must be absent from audits.
- Support authorization/request specs for exact expiry, explicit end, stale MFA, missing reason, and cross-household access.
- Service/job/task specs for natural expiry, repeated processing, concurrent/idempotent retry, and exactly one audit event.
- Documentation contract specs for the new command and evidence fields.

### Verification

- Run each changed focused spec through `task test TEST_FILE=...`.
- Run `task rubocop` for task-scoped quality before committing.
- Commit with a Conventional Commit message describing invitation/RLS and support-expiry closure.

## Task 3: Resolve #1620 — implement household export, retention, offboarding, and purge

### Outcome

An authorized operator can create a complete household-scoped portable export with owned attachments, apply audited retention policy/holds, idempotently offboard a household, and safely purge all tenant-owned data and storage only when no hold applies. Held/offboarded households are denied everywhere else.

### Primary implementation surfaces

- Build on `PortableData::Exporter` and existing data-export controllers rather than creating a second incompatible portable format. Include every portable tenant-owned record and owned Active Storage object without platform secrets, credentials, or audit internals.
- Add durable export lifecycle records for request/generation/download/expiry/failure and audit each transition with identifiers and outcomes only.
- Add deployer-configurable retention policy plus explicit household retention holds containing reason, approver, review date, release state, and immutable audit evidence.
- Add an idempotent offboarding service that atomically disables normal household access and revokes memberships, browser/API sessions, app tokens, OAuth grants, push subscriptions, and native device tokens.
- Add a resumable purge workflow that respects holds, removes every `SchemaInventory` tenant-owned row and household-owned attachment/blob without touching another household, and records partial failure/retry state.
- Enforce offboarded/held household exclusion at central web/API tenant selection, background jobs, notifications, and support-mode boundaries.
- Expose narrowly authorized executable commands/endpoints for export, hold, offboard, and purge; update OpenAPI when an API contract changes.
- Replace procedural runbook prose with exact `task` commands, required inputs, sanitized evidence fields, retry guidance, and safe failure behavior.

### Required RED/GREEN coverage

- Export service/request specs for authorization, completeness, attachment bytes/checksums, cross-household isolation, lifecycle audit states, download, expiry, and failure.
- Retention-policy/hold model, policy, migration, RLS, and audit immutability specs, including approver/reason/review-date requirements.
- Offboarding specs proving immediate denial and revocation of every listed credential/session/device surface, idempotent retry, and no partial re-enable.
- Purge specs covering success, active hold refusal, authorization, retry after injected partial failure, complete `SchemaInventory` deletion, attachment/blob cleanup, audit evidence, and preservation of another household.
- Central boundary specs proving held/offboarded households are excluded from web, API, jobs, notifications, and support.
- Task/runbook documentation contract specs and OpenAPI contract specs where applicable.

### Verification

- Run each changed focused spec through `task test TEST_FILE=...`.
- Run migration/RLS/schema inventory specs and `task rubocop` before committing.
- Commit with a Conventional Commit message describing the hosted household lifecycle.

## Task 4: Resolve #1621 — rehearse and record the hosted backup restore gate

### Outcome

A repeatable operator command restores a current database and attachment backup into an isolated production-like environment, runs migrations as the owner role, runs checks as `med_tracker_app`, verifies tenant/RLS/storage/audit/WORM recovery, and writes a durable sanitized evidence bundle linked from the launch gate.

### Primary implementation surfaces

- Add a repository `task` workflow that accepts explicit backup identifiers/paths, image tag, tester, and durable evidence output location; refuse missing or ambiguous inputs.
- Compose existing `Storage::RestoreVerifier` and `Audit::Verification` commands with new database-role/RLS/tenant-isolation verification. The workflow must prove default denial and cross-household isolation for clinical rows, audit rows, and attachments.
- Keep migration execution owner-only and application verification explicitly under `DATABASE_ROLE=med_tracker_app` with forced RLS.
- Generate a sanitized, machine-readable and human-readable evidence bundle recording date, backup identifier, image/tag, schema version, tester, exact commands, sanitized outputs, failures/remediation, audit/WORM comparison, and final pass/fail outcome.
- Store or publish evidence only to the operator-provided durable location. Do not commit secrets, health data, real tenant identifiers, or transient local evidence to Git.
- Update `docs/operations/hosted-private-beta-runbook.md` and `docs/security/hosted-multi-tenant-hardening-audit.md` with the command, evidence link mechanism, rehearsal cadence, and invalidation triggers.
- Execute the workflow against an available current backup and production-like target. If those external inputs are unavailable, report the exact missing prerequisite and leave #1621 open.

### Required RED/GREEN coverage

- Task/command specs for required inputs, role separation, orchestration order, failure propagation, sanitized evidence output, and refusal to overwrite/claim success after partial failure.
- Verification service specs for default-deny, two-household clinical/audit/attachment isolation, storage checksum, audit-chain verification, and WORM divergence/failure.
- Documentation contract specs for cadence, invalidation triggers, durable evidence link, command, required fields, and redaction rules.
- A real rehearsal evidence bundle from the current backup/environment, outside Git, is required before the issue can be closed.

### Verification

- Run focused task/service/documentation specs through `task test TEST_FILE=...`.
- Run the real restore rehearsal and independently inspect its sanitized evidence bundle.
- Run `task rubocop` before committing.
- Commit automation/documentation only after tests pass; never commit live evidence containing operational or tenant data.
