---
name: team-planning
description: >-
  Plan bounded work for a persistent agent team before implementation. Use when decomposing a
  requested change into tranches, assigning team seats, selecting assistance, or preparing
  file-based briefs, reports, reviews, and a durable progress ledger.
---

<!-- markdownlint-configure-file {"MD013":{"line_length":110,"code_blocks":false,"tables":false}} -->

# Team Planning

Plan first; do not implement or dispatch a writer from this skill. This workflow prepares a
safe, reviewable execution packet for a later tranche-development run.

## Required context

1. Read the current project's team charter and repository instructions.
2. Read `adaptive-model-routing` completely and use it to choose the smallest capable assistance
   level.
3. Use the durable artifacts, independent-review gates, continuous-execution rule, and final broad
   review defined in this skill. No separate development-workflow skill is required.

The charter is the source of truth for project-specific authority, safety rules, product history,
and local commands. Keep those details out of this portable skill.

## Portable vocabulary

The role names in this skill are portable labels rather than project-specific identities:

- Bucky coordinates, integrates, and owns acceptance.
- Nightingale is the sole writer for an active tranche.
- Hubble performs independent read-only review.
- Scout performs bounded read-only discovery.

Luna, Terra, and Sol are model tiers from `adaptive-model-routing`, not team seats. A project charter
may refine these roles, but it must preserve the one-writer and independent-review boundaries.

Medals are post-acceptance recognition rewards. They are never a task, tranche, scope unit, priority,
approval, performance target, or definition of done. Award them only for a distinct accepted outcome
with evidence of correct scope, safety invariants, review response, and reproducible verification.
Do not award medals for attendance, routine completion, failed attempts, rework, or proxy metrics such
as speed, verbosity, test volume, or medal count. Plans must use ordinary work language.

## Plan artifact

Write a file-based plan that gives each tranche:

- a user-visible outcome and definition of done;
- non-overlapping owned paths and explicitly read-only context;
- constraints, safety boundaries, and escalation conditions;
- exact verification commands and the authoritative environment;
- a uniquely named file brief and report path;
- a ledger entry format and the independent-review gate required before completion.

Keep the plan small enough that one writer can carry the whole tranche through review fixes. Split
only at a real ownership or verification boundary; do not turn tightly coupled work into parallel
writer tasks.

## Assistance gates

Choose assistance before producing the plan. Read-only help must have bounded questions, paths,
and a report artifact.

| Work shape | Planning assistance |
| --- | --- |
| Small, exact, low-risk edit with obvious verification | Bucky plans directly; no assistant is needed. |
| Substantial bounded work with independent discovery or review questions | Use read-only Scout or Hubble assistance, routed to Luna `medium`/`high` when available; fall back to Terra at the same effort when Luna is unavailable. |
| High-risk, cross-cutting, safety-sensitive, ambiguous, or repeatedly failing work | Keep judgment with Bucky at Sol `high` or higher; use read-only help only for a sharply bounded evidence question. |

Escalate rather than guess when scope expands, an interface or safety decision appears, evidence
conflicts, or the same verification failure survives two evidence-based attempts.

## Recorded planning routes

When emitting or recording a planning route, list only seats active during planning. Nightingale never
appears in that route, `writer_owner_count` is zero because no active tranche owns writing yet,
and future implementation ownership is not part of a planning decision. Sol `high` and `xhigh`
routes have `fallback: null` while Sol is
available. Luna-to-Terra fallback applies only when Luna is the chosen primary and Luna is
unavailable.

## Planning boundaries

- Never dispatch Nightingale or any other writer while planning.
- Hubble and Scout remain read-only. Their reports inform Bucky's decisions but do not change the
  plan without Bucky's review.
- Do not promise commits, pushes, upstream mutation, or product-scope expansion without the
  authority required by the project charter and repository instructions.
- Preserve existing work, use isolated workspaces and databases when the charter requires them,
  and keep all records truthful.

## Handoff to execution

Before handing a plan to tranche development, confirm that it preserves the durable workflow:
file-based briefs and reports, a progress ledger, independent per-task review, continuous execution,
and a final broad review. Record unresolved questions and their owner in the plan; do not conceal
uncertainty in a vague task.
