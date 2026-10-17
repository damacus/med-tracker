# Documentation index

Canonical table of contents for the Wear OS Specialist guidance.

## Core docs

| Doc | Description |
|-----|-------------|
| [Review checklist](review-checklist.md) | PR/review checklist: product, correctness, UX, performance, data layer. |
| [Performance & battery](perf-battery.md) | Compose, timers, Room, haptics, common watch pitfalls. |
| [Examples](examples.md) | Feature-request and review-comment examples. |

## How-to guides

| Doc | Description |
|-----|-------------|
| [How to review with the checklist](how-to-review.md) | Using the checklist in practice and example PR comments. |
| [How to debug on watch](how-to-debug-on-watch.md) | Reproduce on device, add logs, check main-thread IO and lifecycle. |

## Patterns

| Doc | Description |
|-----|-------------|
| [Timers](patterns/timers.md) | Non-blocking timers, lifecycle, power. |
| [State](patterns/state.md) | Single source of truth, process death, StateFlow. |
| [Haptics](patterns/haptics.md) | When, why, and how to use haptics. |
| [Room queries](patterns/room-queries.md) | “Latest record” patterns, paging vs summaries. |

## Templates

| Doc | Description |
|-----|-------------|
| [Feature plan](templates/feature-plan.md) | Copy-paste template for implementation plans. |
| [PR review](templates/pr-review.md) | Copy-paste template for PR review notes. |

## Cursor & other agents

See **[Cursor & other agents](cursor-and-agents.md)** for:

- How to use [SKILL.md](../SKILL.md) with any agent or IDE.
- How to use the [Experts command](../.cursor/commands/experts.md) in Cursor.
- What is **stable** for tooling (file layout, paths) vs what may change (prompt text, wording).
- Using the repo in other IDEs or agents.
