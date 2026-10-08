# Loco migration working rules

Complete every slice in the approved plan. Migration defects stay in that work;
create follow-up issues only for independent work outside the migration. Preserve
Rails as the runnable rollback application. No migration section is accepted until
its complete acceptance criteria pass.

## Ownership

- The coordinator owns scope, integration, reports, commits and publication.
- Astra owns identity and shared persistence/security changes.
- Nightingale owns care operations and their tests.
- The browser writer owns browser routes, templates and browser tests.
- `verifier_sol` owns builds, dependency installs and disposable test resources.
  The unavailable previous verifier stays stopped.
- Devin CLI `swe-2-max` provides independent capability and security review.

Writers work in disjoint files. Request changes to shared files from their owner;
the coordinator may release a narrow change when that owner is idle. A completed
agent needs a follow-up task to restart. Check its state before handing work back.
Only the coordinator commits or pushes. Use the repository's required identity
and signing rules.

Preserve the integrated main ancestry during publication. A plain rebase can
flatten the merge even when its tree stays correct, delaying hosted checks.
Before pushing, verify that the fetched main tip remains an ancestor of the
candidate; use a merge-preserving update when needed. Verify hosted checks against
the exact pushed head. Do not add empty trigger commits while investigating a
missing check.

## UI review gate

Owner decision, 8 October 2026: every UI change requires three independent
judges before it is called complete: vanilla Astra without design skills,
UI Professional using its skill, and Accessibility using the requested
addyosmani/web-quality-skills accessibility skill. Review the actual desktop
and mobile result, including changed copy and states. Preserve a separate
verdict from each judge. Fix actionable findings through the existing sole
writer and have affected judges recheck the corrected result. Clearly record
unverified behaviour and optional preferences; do not infer accessibility
conformance from screenshots or automated checks. The owner does not require
manual screen-reader testing. These UI judgments supplement the existing
independent code/security review and publication gates.

## Delivery

Use existing public Tasks. Write a failing behavioural test, implement the change,
and run focused checks. Include final test edits in the compilation check before
starting a database fixture. Freeze the tested source for review and verification.
Once a test proves a shared prerequisite is missing, implement the complete related
capability and run its distinct behaviour and security cases together. Do not
restart the application separately for each missing button or form field. Keep
real negative tests and required acceptance checks. Run the full suite once the
complete, reviewed candidate is frozen; repeat it only for changed behaviour or a
new unresolved failure.
Before dispatching a new journey, check its expected behaviour against the latest
explicit user decisions and the current implementation specification. A legacy
reference or earlier draft does not override a newer decision. Replace obsolete
expectations before writing production code; keep the required security outcomes
in the replacement journey. This check uses the existing specification and does
not create a separate research or approval gate.
Run source review alongside focused verification. Resolve actionable findings and
freeze the corrected source before the final whole-suite check. This avoids
finishing a long suite on code already known to need a repair. Publish only after
review and checks pass; changed source needs new applicable evidence.

Before the first final source freeze for a ported capability, compare retained
mandatory inputs, each operation's response fields and flags, replay permissions
and state guards, and the OpenAPI conditional-header contract. Put confirmed gaps
in the existing owning tests and resolve them together. Include the relevant Rails
and OpenAPI excerpts in the first review packet alongside the called helpers.
This uses the existing review and tests, without adding another stage.

Review a usable capability rather than each small repair. Start a fresh Devin
session for a new capability and resume it for corrections. Send actual authorised
diffs and concise evidence; unattended reviews must not need interactive tool
approval. Dan has authorised all source and context needed for Devin reviews;
the existing review-sharing authorisation runs through 16 October 2026. Check findings
against retained behaviour before changing code. Do not silently substitute a
different reviewer if Devin is unavailable.

Include the actual helpers called by the change in the first review packet:
permission checks, projections, input contracts, locked queries and relevant schema
constraints. Reviewers must be able to verify these contracts from supplied source.
Check conditional findings against those implementations before changing behaviour.
This uses the existing packet and review; it adds no separate review stage.
The frozen review packet must name the candidate, changed helpers, exact passing
checks and previous finding dispositions. Batch source reads and reuse unchanged
helper evidence. Do not repeatedly explore upstream or generated data. Preserve
complete security coverage; a shorter packet is not a narrower acceptance scope.

Diagnose one evidenced failure at a time. Separate test setup failures from
application failures. After two unsuccessful fixes, obtain the actual SQL error,
HTTP response or persisted state before another attempt. Reuse a valid owned test
image for source-only retries. Do not repeat a full
suite solely to publish cleanup, a report or a small repair.
Copy large versioned data files directly and verify their provenance and checksum;
do not send whole datasets through a patch tool. If a writer stops making
observable progress without a live job handle, obtain a checkpoint and inspect
the blocked operation. Preserve running builds and tests.
Use the existing browser fixture to attach bounded, redacted application errors
on failure, so the next correction uses the actual rejection rather than another
run solely to retrieve logs. Do not create another runner for diagnostics.
Read the canonical fixture setup before asserting roles, account IDs or other
baseline values. Do not infer them from display names or the operation under test.
Before changing persistence, inspect only facts the change depends on: the existing
SeaORM operation/Loco route, affected constraints/indexes/RLS/grants, and relevant
fixture identity/sequence. Separate verified facts from unanswered questions.
Writer handoffs name the exact reused function, route or constraint in one or two
lines, without a new planning document or runner. Start with the cheapest useful
evidence, then focused behaviour and required combined checks. Reuse models and database
roles when they satisfy the requirement. Add a role or bootstrap step only for a
demonstrated limitation; do not design it around assumed owner permissions.
When several browser cases fail at the same missing entry point, use one case to
prove that prerequisite is missing. After implementing it, run every distinct
behaviour and security case. Do not repeat app startup merely to reproduce the
same prerequisite failure.

## Keep migration machinery out of daily work

Historical relocation, workspace and source-preservation checks belong to explicit
`task migration:audit`, including rollback rehearsal. Do not regenerate historical
ledgers for each capability. Keep real behaviour, permissions, persistence, lint
and resource-cleanup checks in normal application verification.

Reuse Loco features and existing application helpers. Do not add temporary probe
workers, duplicate test runners, per-repair briefs or parallel progress reports.
Remove obsolete helpers alongside their replacement delivery. Keep database probes
that verify real side effects, and isolation that protects unrelated databases.
Delete migration-only helpers when their last caller is replaced. Do not create
a separate cleanup delivery or rewrite a working runner just to remove its name.

Use explicit disposable project names with public Tasks. Inspect diagnostic task
expansion before starting resources. Preserve existing volumes and evidence;
cleanup must target only resources owned by that run. The verifier may generate
lockfiles and formatter output on the owner's request, but may not manually change
shared source. Record the resulting source before acceptance checks.

## Report and continue

Update `/private/tmp/medtracker-migration-status.html` and its Markdown copy at
each completed slice or material blocker. Show published work, current work,
what prevents the next delivery, and the next action. Use plain English. Keep
technical logs separate; avoid duplicate accounts of every verification attempt.
Use application sections that explain what works, what remains and what will prove
completion. Distinguish local, published, accepted, merged and deployed work, and
core production readiness from full migration completion. Continue independent
implementation during source review and hosted CI when ownership is disjoint and
the needed dependencies are locally verified.
Run the requested two-hour retrospective and apply improvements within the user's
authorised scope.

For Loco work, use Loco checks and migrated workflows. Investigate or change Rails
only for a demonstrated compatibility defect or explicit Rails work. Retain
required hosted checks and the final populated rollback verification. Do not
deploy, merge, migrate live data, delete unrelated storage or send unrequested
external messages. Keep incident #2453 and pending approvals visible.
