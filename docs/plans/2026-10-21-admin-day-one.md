# Administration day-one implementation
Spec: GitHub issue #2707, agreed 21 October 2026. This document is the local implementation spec and plan.

## Global Constraints
- Rust/Loco root is primary; Rails is read-only reference. Reuse domain permissions, RLS, last-owner safeguards, transactions and audit. Distinguish household membership actions from global sign-in/platform permissions.
- Preserve API/MCP contracts; use browser-specific typed query/command/result boundaries. SQL performs filtering, sorting and bounded pagination, never full-table Rust filtering.
- Keep useful table functionality without reproducing Rails page count. Accessible labels, keyboard controls, confirmations, errors and mobile access are required. Use existing Tera/DaisyUI/dialog patterns and translation conventions.
- Direct administrator account creation and other-account profile/password editing are deferred. Expired-invitation alert, broad redesign, delivered invitations/audit/platform/dm+d, #2706 and mail work are excluded.
- Fish + RTK + repository Task wrappers; observe meaningful failing tests before executable edits. No broad lint suppression or unsolicited comment edits. Synthetic disposable databases only; no production changes. Do not reset/adopt populated data.
- One implementation writer at a time; implementers never delegate or publish. Controller performs final whole-branch checks and publication after independent task reviews. Before every commit verify author, committer and signing identity; unsigned fallback is per commit only.

## Task 1: Complete household member administration
Build one complete member-management journey in current main, including the existing role editor.
Requirements:
- Member table displays name, email, household role and explicit active/suspended/revoked status, including inactive memberships and useful identities for them. Prefer canonical Account.name and email; when a canonical name is absent and person RLS hides the inactive person, display email as the identity fallback. Preserve existing people RLS; do not broaden clinical/person access for a directory. Follow canonical Account/Person direction.
- Case-insensitive name/email search; household role filter; status filter with all and each state; sortable name/email/role/status columns; deterministic tie-breaks; bounded server-side page size 25 and pagination preserving selected search/filter/sort/language. Validate query inputs safely; do not concatenate raw SQL/order expressions.
- Add suspend, restore and revoke browser membership actions with labelled confirmations and clear success/error handling. Active permits suspend/revoke, suspended permits restore/revoke, revoked permits restore. Role editing stays available where authorised. Use membership identity for lifecycle actions; do not confuse account sign-in activation with membership state.
- Reuse current domain change operation/authorisation/audit transaction and last-active-owner protection. CSRF, fresh authority rechecks, tenant isolation and error observability remain intact. No public API contract change.
- Use typed browser DTO/validated enums/ID/results at new boundaries, and SQL query joins to the canonical account identity for inactive members. Do not redesign untouched domain APIs simply to eliminate legacy JSON.
- Observe focused failing behavioural tests before executable edits; cover search/filter/sort/paging/inactive identities, state transitions, permission/cross-tenant/CSRF/last-owner rejection and audit/persistence. Use existing App::boot/restricted-credential harness. Add focused desktop/mobile browser journey coverage if practical; controller captures manual screenshots.
- Own administration controller/model/view files, related locales and focused tests only; maintain role editor compatibility, existing API callers and care-relationship behaviour. Small focused modules may be added to these existing features. No broad refactors.
- Read root AGENTS, required Rust skills and ADRs. No subagents. Run focused repository Tasks; leave one final full task ci to the controller after both tasks. Check identity before committing explicit owned files with Conventional Commit.
- Write the full implementation/TDD/command/output/self-review report to the supplied report path; return only status, commits, test summary and concerns.

## Task 2: Finish care-relationship list controls
- Read root AGENTS, required Rust skills and ADRs. No subagents. Preserve Task 1 interfaces.
- Expose the existing create relationship form from the household care-relationship list.
- Expose Reactivate for inactive relationships, keep Deactivate for active ones, reuse existing routes/domain commands and confirmation/error conventions. No new pages solely for parity.
- Preserve permissions/RLS, live authority, transaction/audit and existing relationship semantics. Keep keyboard access, labels, confirmation dialogs, errors and mobile access sound; follow translation conventions for added copy.
- Observe focused failing behavioural tests before executable edits; verify create discoverability and inactive-to-active and active-to-inactive transitions, plus relevant rejection/audit behaviour. Reuse current acceptance harness.
- Own relationship templates, related localisation and focused tests; only adjust shared controller code if required, reporting any overlap with Task 1.
- Run focused Task checks; controller owns final full task ci and desktop/mobile visual evidence. Check author/committer/signing identity and commit explicit owned files using Conventional Commit.
- Write the full implementation/TDD/command/output/self-review report to the supplied report path; return only status, commits, test summary and concerns.

## Controller delivery
After each task generate a package and obtain independent spec + quality review, resolve scoped findings using SDD loops. Resolve fixture sign-in using actual harness evidence; do not relax authentication for screenshots. Browser-verify changed screens at desktop/mobile using synthetic data and save screenshots under docs/screenshots/. Run final task ci, task docs:build and git diff --check once candidate is frozen. Security-conscious whole-branch review uses Astra because membership restore/revocation changes access; routine task reviews use Sol. Publish scoped verified branch PR against main, attach artifact, keep issue open pending acceptance/merge, no merge/deploy.

## Current delivery boundary

The member journey is independently deliverable. Care creation/reactivation remains pending an explicit owner decision: current Cedar policy denies delegate_person, and retained-relationship regression coverage deliberately hides creation/reactivation links. The existing operations also affect access grants and guardian authority. Partial care UI changes are preserved privately and are excluded from the member candidate. Member verification and publication do not complete all of #2707.
