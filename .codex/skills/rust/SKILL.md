---
name: rust
description: >-
  MedTracker Rust router for Axum API and browser routes, Leptos UI, SeaORM
  persistence, Rails compatibility, Rust tests, crate selection, performance
  diagnosis and review. Use for work under rust/ or decisions affecting the
  Rust migration. Tailored for GPT-6 Luna, GPT-6.1 Sol and GPT-6 Astra.
---

# Rust

This is a project router, not a Rust tutorial. Repository instructions take
precedence. Read only references whose triggers match the current task.

## Choose what to read

| Trigger | Reference |
| --- | --- |
| First work in an unfamiliar crate, runner or execution environment | [Project map](references/project.md) |
| API route, browser action, Rails parity or authentication change | [Behaviour and access](references/behaviour.md) |
| Database query, clinical write, audit, stock or schedule change | [Persistence](references/persistence.md) |
| Leptos component, browser form, locale or PWA change | [Browser UI](references/browser.md) |
| Test design, failing acceptance case or independent review | [Verification](references/verification.md) |
| Choosing a crate, changing dependencies, profiling or async diagnosis | [Dependencies and performance](references/dependencies-performance.md) |
| Choosing or reassessing model/effort, ambiguous discovery or consequential design | [Model routing](references/models.md) |

Choose from the task's concrete concern, not every topic mentioned in the diff.
Do not load all references at startup. Stop reading when you have enough context
to implement or assess the bounded change. Open another reference only when a
new concern appears. Parallelise independent reads only after selecting them.

Examples:

- A static layout change needs Browser UI; it does not need Persistence.
- A stock action needs Behaviour and access plus Persistence. Load Verification
  when selecting its regression cases, rather than reading the whole handbook.
- A crate comparison needs Dependencies and performance; authentication
  compatibility additionally needs Behaviour and access.

For a mixed Rails/Rust change, use the Ruby router for the Rails-owned part.
Use the relevant OpenSpec skill when the task includes proposal, implementation
tracking or specification changes; this router does not create a second plan.

## Keep guidance worth loading

Assume Luna 6, Sol 6.1 and Astra 6 know Rust ownership, iterators, Result,
generics and standard tooling. Add guidance here only for a recurring project
failure, a non-obvious boundary or a decision that needs evidence. Keep API
examples in current library documentation rather than duplicating them here.

Do not impose blanket rules about clones, trait objects, assertion counts or
type-state patterns. Choose them from the behaviour and evidence at hand.
Commands, flags and supported feature combinations belong to the current
Taskfiles and manifests, not this skill.
