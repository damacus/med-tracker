# Cursor & other agents

How to use this repo with Cursor and other AI-assisted IDEs or agents.

## Skill definition (SKILL.md)

The main entry for any agent or IDE is **[SKILL.md](../SKILL.md)** at the repo root. It defines:

- Scope: Wear OS apps (Kotlin, Compose for Wear OS, coroutines/Flow, MVVM, Room/DataStore, etc.)
- Operating principles (glanceability, tap targets, battery, lifecycle)
- Default implementation workflow and output templates

Point your tooling at the repo (e.g. `npx skills add gitdolucas/wearos-specialist`) so the agent can use the skill when implementing or reviewing Wear OS code.

## Cursor Experts command

The file **[.cursor/commands/experts.md](../.cursor/commands/experts.md)** defines a **Cursor command** that runs a simulated panel of specialists (e.g. Martin Fowler, Kent Beck, Don Norman). Use it when you have a complex, cross-disciplinary question (architecture, trade-offs, product vs tech).

### How to use in Cursor

1. Ensure this repo is available to Cursor (e.g. open the repo or add it as a skill).
2. Invoke the **Experts** command (via command palette or your Cursor commands) and enter your question.
3. The model will respond with a `<reasoning>` section (panel discussion) and a standalone `<answer>` section.

This command is optional; use the Wear OS skill for day-to-day implementation and review, and Experts for deeper or multi-angle decisions.

## What is stable for tooling

If you build scripts or integrations that consume this repo:

| Stable | May change |
|--------|------------|
| Repo root layout: `README.md`, `SKILL.md`, `LICENSE`, `CONTRIBUTING.md`, `CHANGELOG.md` | Exact prompt text inside `.cursor/commands/experts.md` |
| Presence and path of `docs/` and `docs/index.md` | Wording of checklist items or examples (content may be refined) |
| Paths to core docs: `docs/review-checklist.md`, `docs/perf-battery.md`, `docs/examples.md` | Number and names of pattern docs under `docs/patterns/` |
| Skill name and description in `SKILL.md` front matter | |

We avoid breaking changes to file paths and to the fact that `SKILL.md` and `docs/` exist; we may add files or tweak content within those files.

## Other IDEs and agents

For IDEs or agents that don’t use Cursor commands:

- Use **SKILL.md** as the primary reference for Wear OS behavior and templates.
- Use **docs/** for the full set of checklists, how-tos, patterns, and templates.
- The Experts command is Cursor-specific; other tools can still use the rest of the repo as a static knowledge base.
