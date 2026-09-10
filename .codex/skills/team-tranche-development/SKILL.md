---
name: team-tranche-development
description: >-
  Execute one approved, bounded persistent-agent-team tranche through implementation, independent
  review, fixes, verification, and handoff. Use after team-planning has produced file-based briefs
  and when the project charter defines seat ownership.
---

<!-- markdownlint-configure-file {"MD013":{"line_length":110,"code_blocks":false,"tables":false}} -->

# Team Tranche Development

Execute one approved tranche continuously until it is complete or genuinely blocked. Read the
current project's team charter and repository instructions before acting.

## Required upstream workflow

Read and apply `adaptive-model-routing` completely. It governs risk-based judgment and model
selection. This skill defines its own file briefs and reports, progress ledger, independent review,
continuous execution, and final broad review; no separate development-workflow skill is required.

## Portable vocabulary

The role names in this skill are portable labels rather than project-specific identities:

- Bucky coordinates, integrates, and owns acceptance.
- Nightingale is the sole writer for an active tranche.
- Hubble performs independent read-only review.
- Scout performs bounded read-only discovery and may run deterministic verification or poll remote
  checks without changing source files.

Luna, Terra, and Sol are model tiers from `adaptive-model-routing`, not team seats. A project charter
may refine these roles, but it must preserve the one-writer and independent-review boundaries.

## Charter override: persistent writer ownership

The project charter overrides generic SDD's fresh-implementer/fix-subagent pattern for a tranche:

- One Nightingale owns all product and test writing for the entire active tranche.
- That same Nightingale handles every review fix and re-verification until the tranche passes review
  or formally hands off under the charter.
- Hubble and Scout do not edit source files; they may investigate, independently review, run
  deterministic verification, or poll remote checks.
- Bucky owns product interpretation, integration, acceptance, commits, and delivery.

Do not introduce a replacement writer mid-tranche. If Nightingale cannot continue, stop at a safe
boundary and follow the charter's handoff process; Bucky decides the next authorized assignment.

When two evidence-based Nightingale fixes fail, the active route remains `team-tranche-development`.
Bucky pauses Nightingale's implementation for Sol `xhigh` judgment, but Nightingale retains the tranche's
sole write ownership (`writer_owner_count: 1`) and is not replaced. The selected route seats are
Bucky and Nightingale; do not add Hubble or Scout unless Bucky separately assigns a bounded read-only
evidence question. Keep approval code `bucky_sol_xhigh` and escalation code
`same_nightingale_no_scope_broadening`.

When a parallel-writer proposal appears inside an active tranche, adjudicate it in
`team-tranche-development`; never reopen `team-planning`. Bucky coordinates the existing route,
Nightingale remains the sole write owner (`writer_owner_count: 1`), and Hubble and Scout remain
read-only support. All four seats continue in their existing roles, rather than receiving new
assignments. Use Sol `high`, not `xhigh` unless another high-risk trigger applies, approval code
`serialize_writers`, and escalation code `no_parallel_writer`.

## Execute the tranche

1. Read the approved plan, relevant file brief, current progress ledger, and any current handoff.
   Treat ledger-complete tasks as done after corroborating them with Git and review evidence.
2. Have Bucky validate scope, ownership, safety constraints, and verification before Nightingale writes.
   Nightingale works only in its owned paths and preserves unrelated work.
3. Nightingale writes a detailed file-based report: implementation, changed files, exact checks,
   self-review, concerns, and the next safe action. Nightingale returns only a compact status summary.
4. Obtain an independent Hubble review with separate requirements and code-quality verdicts.
   Critical and Important findings return to the same Nightingale, which fixes, re-tests, updates the
   report, and obtains re-review. Resolve any "cannot verify from diff" item before completion.
5. Bucky records a truthful completed task in the progress ledger only after the independent review
   is clean. Continue through the approved tranche without pausing for routine human check-ins.
6. After all tranche tasks are clean, obtain a broad independent whole-tranche review. Nightingale fixes
   its Critical and Important findings, obtains re-review, then Bucky rechecks the evidence before
   final acceptance.
7. Bucky alone integrates, commits, pushes, and delivers when authorized. Nightingale does none of
   those integration actions unless the charter is explicitly changed.

## Separate judgement from command execution

Do not spend a high-judgement model's context on long deterministic commands or repeated status
polls. When tests, builds, or remote checks are expected to take more than one minute, Bucky may
assign Scout on Luna `medium` or `high` as a non-writing verification runner. Give that runner only
the command, working directory, expected evidence, stopping conditions, and known environmental
constraints; a full implementation context is unnecessary.

The runner reports the exact command, exit status, duration, concise failure summary, and paths or
URLs for retained output. It does not diagnose ambiguous failures, change files, relax gates, or
decide acceptance. Bucky or the selected judgement model interprets the evidence. Keep quick focused
commands with the active owner when delegation overhead would exceed the likely saving.

For remote CI, let the Luna runner poll compact status endpoints and return only state changes,
failed job details, and final timings. Avoid streaming complete logs into Bucky's context. Fetch the
smallest relevant failed-step log only after a failure appears.

## Expensive-feedback gate

Treat a full suite or remote CI run as an acceptance gate, not the default diagnostic loop:

1. Map every issue acceptance criterion to focused local evidence, full-suite evidence, or
   remote-only evidence before implementation.
2. Run the smallest contract or reproduction that can disprove the change first.
3. Before starting a gate expected to exceed ten minutes, have the selected judgement model inspect
   the diff, focused evidence, environment variables, artifact paths, and failure propagation.
4. Start the expensive gate only when it answers a remaining acceptance question. If the remote
   environment is the only authority, state the exact question the run will answer.
5. After a failure, inspect and reproduce the failing boundary before starting another full run.
   Rerun unchanged only when evidence supports a transient infrastructure or test failure.

For CI performance work, local timings are directional evidence rather than remote acceptance.
Collect the current remote baseline before finalizing the design, and compare the changed run in the
same job environment. Environment-gated coverage, sharding, caching, and artifact behaviour must have
an executable contract using the exact job variables and isolated temporary output paths.

## Assistance and model gates

Use the approved plan's assistance level, then reassess at each handoff:

| Work shape | Execution route |
| --- | --- |
| Small, exact, low-risk implementation | Nightingale may proceed directly after Bucky's scope check; use no extra assistant unless an independent review is required. |
| Substantial bounded implementation or routine review | Use the smallest capable route from adaptive-model-routing. Prefer Luna `medium`/`high` for mechanical read-only help when available; use Terra at the same effort when this runtime does not expose Luna. |
| High-risk, cross-cutting, safety-sensitive, ambiguous, or repeatedly failing work | Bucky keeps the decision at Sol `high` or higher, narrows the tranche or escalates, and may request only bounded read-only evidence from Hubble or Scout. |

Escalate when scope exceeds ownership, repository behavior contradicts the brief, a public or
safety contract appears, evidence conflicts, or the same verification failure survives two
evidence-based Nightingale attempts. Escalation is not permission to bypass the one-Nightingale rule.

Reassess the judgement model before Nightingale starts, when the implementation shape changes, and
before each expensive-feedback gate. Escalate at the first evidence of architectural ambiguity or
interacting failure semantics; do not wait for two failed implementation attempts when the problem
has already stopped being bounded execution.

Use Astra `medium` directly for exceptional CI judgement such as interacting workflow fan-in,
coverage collation across shards, cross-runtime compatibility, or failure propagation whose next
feedback cycle is expensive. Astra should define or review the contract and hand a bounded
implementation back to Luna when the remaining work is objective. This is targeted judgement, not a
reason to use Astra for command execution, routine edits, polling, or already-specified fixes.

## Records and safety

- Use uniquely named file briefs, reports, review packages, and handoffs; do not rely on chat
  memory for durable state.
- Medals are post-acceptance recognition rewards. They are never a task, tranche, scope unit, priority,
  approval, performance target, or definition of done. Award them only for a distinct accepted outcome
  with evidence of correct scope, safety invariants, review response, and reproducible verification.
  Do not award medals for attendance, routine completion, failed attempts, rework, or proxy metrics such
  as speed, verbosity, test volume, or medal count. Plans must use ordinary work language.
- Preserve the charter's authority, safety, moral, and external-action boundaries. Never falsify
  test, review, ledger, or handoff evidence.
- A blocker is genuine only after safe, in-scope checks and alternatives are exhausted. Record the
  evidence, owner, and next action rather than silently broadening the work.
