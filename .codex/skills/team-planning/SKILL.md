---
name: team-planning
description: >-
  Plan bounded, complete user journeys before implementation. Use when decomposing
  requested work into slices, assigning ownership and defining acceptance evidence.
---

<!-- markdownlint-configure-file {"MD013":{"line_length":110,"code_blocks":false,"tables":false}} -->

# Team Planning

Prepare an execution brief for [team-slice-development](../team-slice-development/SKILL.md).
Do not implement or dispatch a writer while planning. Scale preparation to the work;
a clear small change does not need a team, questionnaire or new tracker.

## Required context

Read repository instructions, the applicable charter, approved decisions and current
handoff. Inspect existing behaviour before proposing a replacement. Read and apply
`adaptive-model-routing` and its reference for all model, effort, usage, availability
and escalation decisions. Do not duplicate routing policy or prescribe models by role.

The charter supplies project authority, safety rules and commands. The latest explicit
owner decisions take precedence over older plans. Resolve material contradictions before
dependent work, while continuing independent investigation.

## Choose useful slices

Define each slice as a complete usable journey or independently verifiable technical
outcome. Include UI, operations and resulting state where applicable. Split only at a real
ownership or verification boundary. Keep tightly coupled API, UI and integration together.

Keep one accountable leader and one source writer per active slice through fixes and
delivery. One agent can fill both roles. Bucky, Nightingale, Hubble and Scout are optional
charter names for responsibilities, not a mandatory team. List only roles actually needed.

Parallel work must have independent owned paths and a viable verification environment.
Identify owners for shared files, integration and scarce test capacity before dispatch.
Do not manufacture additional work to keep agents occupied.

## Write one brief

Reuse the existing plan or slice record. Include only what execution needs:

- User-visible outcome, exclusions and observable completion criteria.
- Existing implementation and assets to reuse, with relevant source paths.
- For migrations, the reference journey, baseline evidence and approved differences
  required by team-slice-development; preserve behaviour and presentation by default.
- Source owner, checkout, owned paths, read-only context and dependency owners.
- Focused checks, final gates, required review and authoritative evidence.
- Unresolved decisions, escalation conditions and next implementation action.
- For delegated work, where session IDs, live jobs, outputs and completion state belong.

The owner should not have to redescribe an existing application. Discover its rules and
UI from source and runtime evidence. Ask focused questions only when contradictions or
consequential choices remain. Separate migration parity from optional redesign.

## Assistance and handoff

Use the routing policy to judge uncertainty, consequence and verification quality.
Migration discovery, bounded replication and compatibility acceptance can need different
levels of judgement. No fixed model is required to lead every migration.

Read-only assistance needs a bounded question and retained result, and must save more
than briefing and coordination cost. Delegation and external sharing require existing
authority; planning does not grant it.

Before handoff, check that the brief provides the reference and evidence needed to accept
the whole journey. Require independent review for non-trivial migrations and security work,
plus repository-mandated gates. Do not prescribe duplicate per-task and whole-slice reviews.
Record outstanding decisions honestly and pass execution to team-slice-development without
reopening settled scope.
