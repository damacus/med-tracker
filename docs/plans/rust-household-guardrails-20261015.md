# Rust household architecture and guardrails delivery

## Objective and authority

Deliver the user's four-step cleanup/tooling/skills/PR request, including the
subsequently approved application-wide refactor. The latest owner decisions
below supersede earlier Rails-parity plans and the initial proposal. Record
architectural decisions as ADRs, not a new OpenSpec change.

## Global Constraints

- Branch: `chore/codex-skills-and-rust-guardrails`; worktree:
  `/private/tmp/medtracker-rust-guardrails`. One implementation writer at a time.
- Include all initial local changes, including deletions and untracked files.
  The original checkout remains untouched. Inspect and fix invalid resulting
  configuration or command examples rather than publishing known breakage.
- Loco 1.2 is primary; Tera/DaisyUI views; PostgreSQL 18. Rails is read-only
  reference code. No legacy data import, credentials migration, database parity,
  reverse migration or production deployment. Fresh-schema changes must never
  reset or silently adopt an existing populated database.
- Account holds authentication, profile information and preferences, and has
  exactly one person. A person optionally has an account. No separate user or
  profile table. People may belong to multiple households and exist without
  accounts. Minors do not have logins. Preserve existing capacity rules.
- A household owns medication inventory and stock. Membership permits viewing
  and recording household care; managers additionally administer membership.
  Remove the old per-person view/record/manage gate within a household.
- A person can create a household, or accept a manager's invitation to join.
  Adult person or authorised guardian approves sharing an existing person's
  care with another household. Do not infer guardianship from management alone.
- Active household is per session/device, restricted to current memberships.
  Every form/write remains bound to its original household after switching.
  Reject revoked membership at every request and mutation boundary.
- Locations are optional physical storage labels within a household, with no
  membership or permissions. Household inventory works without creating a label.
- Personal care follows the person. Current households receive current
  treatments, schedules, active pauses and latest administration status, not
  automatically the detailed history or narrative notes of another household.
  The person keeps access to full personal history and controls additional
  disclosure. On leaving, the old household keeps events recorded there and
  immutable attribution, but receives no future updates.
- Explicit stock transfer is initiated by the moving person or authorised
  guardian and limited to stock allocated to that person. Communal stock stays
  put. Transfer exact selected quantities atomically and idempotently; source
  and destination must be valid household participations. Never auto-move stock.
- Reuse maintained security/protocol libraries. Preserve live permission checks,
  RLS, audit attribution, replay protection, atomic stock deduction and mail
  outbox. Better Auth has not been proved faulty; its organisation adapters must
  not create a second source of membership truth. Retain atomic domain operation
  boundaries wherever the pinned plugin cannot supply them.
- Typed DTO -> validated command -> operation -> typed result -> DTO boundaries
  throughout root src. Distinct entity ID newtypes, enum dose sources, typed
  conflict codes, parsed timezone-aware timestamps and exact Decimal math.
  JSON remains legitimate at transport/configuration/template/document-storage
  boundaries; no internal struct -> Value -> struct command passing.
- TDD for executable changes. Use Task commands and Fish/RTK. Do not alter code
  comments unless needed for requested guardrails/documentation or lint fixes.
  No broad lint suppressions, including in tests or generated entities. Fix every
  violation of the requested policy while retaining meaningful assertions.
- Preserve API/MCP contracts except explicitly superseded access/location
  semantics; document deliberate changes in OpenAPI, clients and ADRs together.
- No merge/deploy/live data actions. Commits and feature-branch push/PR are
  authorised. Verify author, committer and signing identity before every commit;
  use commit-local signing fallback only if 1Password is unavailable.

## Task 1: Record ADRs and install Rust-specific agent guidance

Create sequential ADRs after existing 0012, following docs/adrs conventions:
accounts/people/households; household inventory and personal care continuity;
authentication/membership/tenancy responsibilities; fresh Rust without legacy
compatibility. Record the exact global constraints, alternatives, consequences,
superseded old assumptions and accepted decision versus implementation status.
Keep unresolved organisation-plugin replacement details Proposed, with the
known pinned invitation-acceptance separate-write limitation and current secure
adapter. Do not claim new application behaviour already works. Link the ADRs
from design documentation and documentation navigation.

Remove obsolete CLAUDE.md, .claude and .jules; consolidate root AGENTS.md and
delete agents.md. Move Ruby/Rails-specific rules to rails/AGENTS.md, marking
Rails read-only. Root guidance must be Rust-only and require the three local
skills. Create .agents/skills/rust-loco-architecture/SKILL.md,
rust-migration-and-tooling/SKILL.md and rust-drift-audit/SKILL.md with the user's
requested names/descriptions and complete actionable rules, corrected for the
latest decisions: no legacy import/parity requirement, no duplicate profile
table, household membership replaces intra-household per-person grants. Retain
Rust-native portability side-effect invariants and property-test requirements.
Preserve and review all initial local changes, correcting invalid mise TOML and
README executable names. Retain legacy rust crates; document actual consumers
and superseded application status. Update narrowly affected tests/references
that require duplicate guides or removed files, first proving relevant failures.
Run documentation build, focused tooling checks and diff whitespace checks.
Commit all Task 1 changes only; leave lefthook recreation for Task 4 explicitly.

## Task 2: Implement accounts, household participation and active tenancy

Read the accepted ADRs and current Loco boot/router/test harness. Replace the
fresh identity/tenancy schema without retaining the users/profile chain.
Accounts have one unique person reference; people have optional reverse account
association and many household participations. Preserve account-level profile
settings. Make login registration/onboarding, household creation, invitation
acceptance, membership removal, household list/selection and actual browser/API
journeys work on this model. Keep an authoritative membership store and reuse
Better Auth for the standard capabilities it provides. Do not bypass atomic
domain operations for invitation acceptance or duplicate role rules. Expose
whole-household member care access and manager-only member administration.
Represent guardian authority separately from household role; fail closed until
an adult/guardian has approved sharing. Remove obsolete person grants only
after all callers, RLS and tests are migrated. Session/device selection must not
retarget open forms. Update fixtures and model mappings with schema changes.
Write red real-router/PostgreSQL acceptance for accountless minors, unique
account/person, multi-household membership, authorised invitations, invalid and
revoked selections, cross-tenant denial, manager/member roles, and transactional
failure. Exercise desktop/mobile UI with retained styling and save screenshots
for visible changes. Verify focused gates and commit. Do not claim care-summary
or stock-transfer support until Task 3 integrates it.

## Task 3: Implement personal care continuity, stock ownership and typed operations

Implement approved care-summary visibility and person-controlled historical
disclosure. Separate person-owned treatment definitions/history from
household-owned inventory, preserving original event household and acting
person plus account credential attribution. Leaving revokes future access,
retains local historical events, and preserves the person's own full history.
Make locations optional stock labels and remove location membership/access
logic, updating API/client contracts for changed semantics.
Implement explicit assigned-stock transfers by the person/guardian with exact
amounts, atomic dual-sided inventory/audit writes, concurrency protection and
idempotency. No communal stock appropriation, negative inventory or automatic
transfer when joining/leaving. Preserve dose replay, latest shared dose status,
atomic outbox and actor provenance across households.
Apply typed command/result boundaries and entity ID wrappers throughout root
application domains and callers, not just dose writing. Remove internal JSON
round-trips and stringly domain codes/timestamps/amounts. Centralise privacy-safe
DbErr classification/tracing with generic responses and correlated request IDs.
Move SQL-worthy filtering to query boundaries. Bound CPU-heavy blocking work
using app-owned semaphores and keep transports/operators scoped to AppContext
lifecycles. Keep Rust-native export/restore safe; do not add old-system import.
Write failing acceptance/property tests for data-sharing and revocation,
double-dose replay/stock, restore without side effects, DST transitions, transfer
atomicity/concurrency, cross-household isolation and privacy-safe diagnostics.
Verify real API/browser journeys and commit integrated functionality.

## Task 4: Enforce strict Rust linting and nextest across local and hosted gates

Add inherited workspace rust/clippy lints for root, migration and all client
tools packages. Forbid unsafe_code. pedantic warn priority -1; deny dbg_macro,
todo, unimplemented, float_cmp, mem_forget and valid loss/precision conversion
lints (the requested lossy_float_conversions is not an existing lint; use
cast_precision_loss, cast_possible_truncation, cast_sign_loss and
lossy_float_literal). Warn unwrap_used, expect_used, indexing_slicing,
needless_pass_by_value and lowercase implicit_clone. Fix all resulting warnings
including test/generated code; do not suppress them or neuter tests.
Install pinned cargo-nextest, cargo-machete and cargo-deny through Mise. Add
minimal deny.toml covering advisories, duplicate-version warnings and sources;
allow only actually required pinned Git sources. Dependency security fixes must
be proven, not ignored. Keep formatting non-mutating in gates.
Update root/client Taskfiles, CI workflows and recreated lefthook.yml to run
fmt check, clippy --workspace --all-targets --all-features -- -D warnings,
machete, deny advisory/bans/sources checks, nextest --no-tests=fail and separate
cargo test --doc. Adapt current owned-database/storage preparation, sharding and
compiled-artifact reuse rather than bypassing them or running tests twice.
Cutover rehearsal remains explicit and tests fresh Rust save/restore, with no
Rails compatibility requirement. Fail zero-test selections; preserve browser
acceptance and cheap-first stage order. Add failing runner/CI regression tests,
run focused gates then complete relevant suites, and commit.

## Task 5: Integrated review, acceptance and publication

Resolve cross-task findings, update docs/contracts and confirm accepted ADRs
match code. Run task ci plus client tools gates and docs build on a frozen
candidate. Run explicit fresh Rust backup/restore rehearsal against disposable
owned data. Compare visible desktop/mobile journeys, not just screenshots.
Obtain broad independent review with complete branch diff, ADRs and evidence;
fix verified findings and re-review changed boundaries. Review all initial local
edits are included without invalid config/executable examples. Commit remaining
in-scope fixes, push feature branch, create and verify one PR against main.
PR description explains behavioural changes and retained legacy rust consumers;
omit routine CI test sections. Report published/hosted/merged/deployed states
accurately and identify any unfinished required acceptance rather than marking
partial work complete.
