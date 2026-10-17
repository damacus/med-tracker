# Wear OS Specialist

Wear OS–focused implementation, review, and debugging guidance for smartwatch apps. Use when building or reviewing Wear OS apps, improving wearable UX, or addressing watch performance, battery, and lifecycle issues.

## Who this is for

- Android developers shipping **Wear OS** apps (Kotlin, Jetpack Compose for Wear OS, coroutines/Flow, MVVM, Room/DataStore).
- Teams that want consistent **review checklists** and **performance/battery** notes.
- Anyone using Cursor or other AI-assisted IDEs and want a Wear OS–aware skill/agent.

## Quickstart

Install the skill so your IDE or agent can use it:

```bash
npx skills add gitdolucas/wearos-specialist
```

Or clone the repo and point your tooling at the local path:

```bash
git clone https://github.com/gitdolucas/wearos-specialist.git
# Then configure your editor/agent to use this repo (e.g. Cursor skills, rules).
```

## What you get

| Asset | Description |
|-------|-------------|
| **Skill definition** | [SKILL.md](SKILL.md) — scope, operating principles, implementation workflow, output templates. |
| **Review checklist** | [review-checklist.md](review-checklist.md) (also [docs/review-checklist.md](docs/review-checklist.md)) — PR review checklist for product, correctness, UX, performance, data layer. |
| **Performance & battery** | [perf-battery.md](perf-battery.md) (also [docs/perf-battery.md](docs/perf-battery.md)) — Compose, timers, Room, haptics, common pitfalls. |
| **Examples** | [examples.md](examples.md) (also [docs/examples.md](docs/examples.md)) — feature-request and review-comment examples. |
| **Expert panel command** | [.cursor/commands/experts.md](.cursor/commands/experts.md) — Cursor command that runs a simulated panel of specialists (see [Cursor & other agents](#cursor--other-agents) below). |

Full table of contents: [docs/index.md](docs/index.md).

## Using in Cursor vs other agents

- **Cursor**: Add this repo as a skill (e.g. via `npx skills add` or your Cursor skills config). Use the **Experts** command (if enabled) for complex, cross-disciplinary questions; use the Wear OS skill when implementing or reviewing watch-specific code.
- **Other IDEs/agents**: Consume [SKILL.md](SKILL.md) and the `docs/` content as reference. File paths and the structure described in this README are kept stable for tooling; prompt formats in `.cursor/commands/` may evolve.

See [Cursor & other agents](docs/cursor-and-agents.md) in the docs for usage and compatibility notes.

## Docs and patterns

- [Documentation index](docs/index.md)
- [How to review with the checklist](docs/how-to-review.md)
- [How to debug on device](docs/how-to-debug-on-watch.md)
- [Patterns](docs/patterns/): [Timers](docs/patterns/timers.md), [State](docs/patterns/state.md), [Haptics](docs/patterns/haptics.md), [Room queries](docs/patterns/room-queries.md)
- [Templates](docs/templates/): [Feature plan](docs/templates/feature-plan.md), [PR review](docs/templates/pr-review.md)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to propose changes and our content guidelines.

## License

[Apache-2.0](LICENSE).
