# Experts command

Run a simulated panel of specialists for complex, cross-disciplinary questions (architecture, trade-offs, product vs tech).

## When to use

- You have a complex problem at the intersection of software, business, and technology.
- You want multi-expert analysis, roundtable discussion, or "ask the experts" style reasoning.
- You need structured reasoning (expert tags) plus a standalone answer with summary, rationale, recommendations, and risks.

## How to run

1. In Cursor, open the command palette and run the **Experts** command (or use the agent skill "experts-panel" if configured).
2. Enter your question or describe the decision you’re weighing.
3. The response will include a `<reasoning>` section (panel discussion) and a standalone `<answer>` section.

## Relation to this repo

This command is optional and separate from the Wear OS Specialist skill. Use the Wear OS skill for implementation and review of watch-specific code; use Experts for deeper or multi-angle decisions (e.g. architecture, product trade-offs, cross-cutting concerns).
