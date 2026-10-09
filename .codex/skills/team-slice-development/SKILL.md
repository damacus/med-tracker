---
name: team-slice-development
description: >-
  Deliver one approved, complete user journey through implementation, review, fixes,
  verification and publication. Use for bounded team work and behaviour-preserving
  migrations; retain one accountable owner and follow delegated work to completion.
---

<!-- markdownlint-configure-file {"MD013":{"line_length":110,"code_blocks":false,"tables":false}} -->

# Team Slice Development

Finish one approved slice before expanding scope. A slice delivers a complete usable
journey or an independently verifiable technical outcome. Include the UI, operations and
resulting state when the journey has them. Read the repository instructions, approved
decisions, current handoff and applicable team charter.

## One model policy

Read and apply `adaptive-model-routing` and its reference. It alone governs model choice,
effort, usage checks, fallbacks and escalation. Do not copy its routing tables here or
assign a model because of a team title. The thread leader is not automatically Astra.

At the start, material scope changes and failed acceptance, apply that policy to the
actual uncertainty and consequence. Briefly record the decision in the existing slice
record: unresolved question, chosen route, why it fits and evidence needed to proceed.
A recommendation does not switch the running model; verify the actual selection using
the supported mechanism. Preserve explicit user model choices and required permissions.

For migrations, judge discovery, implementation and acceptance separately:

- Establishing undocumented behaviour, resolving conflicting contracts, authentication,
  data preservation and cutover can require substantial judgement before any code.
- Replicating a proved journey with established patterns and objective checks can use
  a cheaper route. A large migration does not make every edit an architecture decision.
- UI similarity is not an objective check until the reference and comparison are defined.
  Treat an unverified reference or unexplained difference as unresolved evidence.

When a routing trigger appears, pause the affected implementation and reassess before
another attempt. Resolve the question through better evidence, narrower work, a model
or effort change, or an owner decision; verify that the intervention helped. Do not
repeat a failed approach or start extra agents merely to appear active.

## Ownership without a standing team

The leader owns interpretation, integration, acceptance and authorised publication.
A single implementer owns source and test changes for the slice, including review fixes.
These may be the same agent; do not create an extra role for work it can finish directly.
Reviewers and discovery helpers remain read-only.

Existing charters may call these roles Bucky, Nightingale, Hubble and Scout. These are
responsibilities, not four mandatory agents. Review follows consequence, the routing
policy and repository requirements. Independent review is required for non-trivial
migrations and security-sensitive work.

Keep one thread responsible through fixes and delivery. Split parallel slices only when
their ownership and verification are independent; shared files and test capacity must
have explicit owners. Do not split a tightly coupled journey into API, UI and integration
threads. Before transferring ownership, stop the previous writer and record the exact
checkout, branch, source revision, uncommitted work, live jobs and next action. The receiver
acknowledges ownership; do not leave two writers or an ownerless candidate.

## Preserve the application during migration

The existing application and latest explicit owner decisions define the intended result.
Migration is not permission to redesign. Preserve behaviour and presentation unless a
specific difference is approved. A framework change does not authorise a product change.

Before implementing each journey:

1. Inspect its source, tests, assets and running UI. Identify permissions, validation,
   persisted effects, navigation and important failure states. Resolve conflicts between
   schema, fixtures, API documents and actual behaviour; do not assume one proves parity.
2. Capture a reusable reference for this journey: source revision, synthetic fixture,
   user role, locale, timezone, theme, viewport and relevant desktop/mobile screenshots.
   Retain it in the existing slice record or repository evidence location.
3. Inspect and reuse existing fonts, icons, images, CSS, theme tokens, component markup,
   spacing and responsive behaviour where compatible. If adaptation is necessary, state
   the concrete limitation and preserve the observable result. Do not guess replacements.
4. Record approved differences beside the reference. Ask only about genuine contradictions
   or consequential changes; do not ask the owner to respecify facts available in the app.
   Do not silently preserve a discovered security defect or invent its replacement policy.
5. Derive failing acceptance tests from the reference before production code, following
   repository TDD rules. Reuse equivalent scenarios across old and new runtimes where
   practical, with isolated data. Tests invented solely from the replacement are insufficient.

If the reference cannot run, inspect retained evidence and continue independent work.
State the missing proof; do not claim visual or behavioural parity until it is resolved.
Do not reopen settled framework or product choices without new evidence.

## Execute and accept the slice

1. Confirm the approved outcome, exclusions, owner, reference, approved differences and
   required checks in one existing brief. Reuse valid evidence; avoid duplicate trackers.
2. Implement the smallest complete journey. Reuse established libraries and framework
   capabilities; do not turn a migration into a new framework or component-library project.
3. Run focused tests during development. Verify saved state and permissions, including
   denied, empty, invalid and error cases relevant to the journey.
4. Exercise the actual replacement UI against the reference with equivalent data and
   settings. Compare desktop/mobile layout, content hierarchy, fonts, themes, navigation,
   dialogs, buttons, keyboard/focus behaviour, validation, save/cancel and resulting state.
   Use side-by-side images or overlays as useful; inspect differences and record verdicts.
5. Obtain the required independent review of the integrated journey, reference, approved
   differences, diff and evidence. Review correctness and usability, not only source style.
   The leader remains responsible for reconciling findings with current code.
6. The same implementer fixes in-scope defects. Re-review affected behaviour and boundaries.
   Repeat broad review only for changed scope, shared contracts or invalidated evidence;
   do not require both per-task and whole-slice reviews of unchanged work.
7. Run all applicable acceptance gates on the final candidate. Report completion only
   when required behaviour, visual comparison, review and verification are satisfied.

A screenshot captured is not a screenshot compared. Test counts, rendered routes and
placeholder pages do not establish completion. Do not approve new screenshot baselines,
weaken assertions or hide controls to make a mismatch pass. Every material difference
must be fixed or explicitly approved. Keep discovered in-scope defects in this slice;
filing an issue does not make the journey complete.

## Follow delegated work to a result

Delegate only within existing authority and when savings exceed briefing and integration
cost. External workers such as Devin retain the same scope, reference and acceptance
criteria; sending them work does not transfer the leader's accountability.

- Give one bounded job, owned paths, prohibited actions, required evidence and stopping
  conditions. Check necessary tool access and command permissions before a long run.
- Record the stable external session ID, process/job handle, checkout, source revision,
  output location and responsible owner in the existing slice record.
- Before leaving work unattended, establish and verify a completion/failure notification
  mechanism. If none exists, remain responsible for bounded status checks while active;
  do not promise monitoring after the turn ends. Scheduling requires authorisation.
- Detect command rejection, requests for input, process exit and missing progress.
  Read the result before reporting success. Silence and an observation timeout do not
  prove continued work, failure or a need to start a duplicate session.
- Resume the same session when appropriate. Retrieve its diff and evidence, inspect them,
  complete integration and verification, and report accepted, blocked or incomplete status.

## Keep verification proportionate

Use project-native commands and changed-area rules. Start with the smallest reproduction
that can disprove the change. Full suites and hosted CI are acceptance gates, not routine
diagnostic loops. Before an expensive run, inspect the diff, focused evidence, environment,
output paths and failure propagation; identify the remaining question it answers.

Freeze the candidate during verification and check the integration base before final
gates. Diagnose a failure before rerunning; unchanged retries need evidence of a transient
failure. Reuse valid caches and results for unchanged inputs. Do not skip required checks.

A separate verification runner is optional. Use one only when it saves context or time,
with model selection delegated to the routing policy. Retain exact job handles and concise
exit status, duration, failures and output paths. Prefer completion events and compact
status changes over repeated agent conversations or full-log polling.

## Deliver and retain evidence

Keep one concise record of the outcome, source revision, approved differences, checks,
review and UI comparison, unresolved work, live jobs and next action. Update it rather
than creating a report for every agent exchange.

The leader completes authorised commits, pushes and PR publication after required gates.
Distinguish local, reviewed, published, hosted acceptance, merged and deployed states.
Never silently defer an in-scope defect or claim a migration complete from a foundation.
Respect explicit merge, deployment, data-loss and external-sharing boundaries.

Reassess the workflow when owner corrections, escaped defects, repeated runs or handoff
cost increase. Use measured completion cost where available; do not infer allowance
savings from API prices. No medals, test counts or agent counts define success.
