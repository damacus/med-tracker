---
name: rust-drift-audit
description: >-
  Audits pull requests, modules, or migration diffs for drift from accepted fresh Rust decisions, stringly-typed Rust anti-patterns, swallowed database errors, and resource lifecycle bugs. Use when reviewing code, auditing PRs, or planning refactors.
---

# rust-drift-audit

Read root AGENTS.md and ADRs 0013–0016 before applying this skill.
These are accepted direction, not evidence that pending application work passes.

- Search Value/json!/.as_str in src/models, src/operations, src/services, flag
  actual internal round trips; legitimate transport/config/document/Tera JSON
  is not a finding just because a search matches.
- Flag domain String timestamps/decimals/enums/conflict codes or interchangeable
  raw i64 identifiers, not necessary raw persistence columns and wire DTOs.
- Flag new adapters solely mirroring Rails/Rodauth tables without external need.
- Flag map_err(|_| ...)/From<DbErr> losing categories without safe tracing.
- Flag unbounded blocking work, process-wide HTTP clients tied to short Tokio
  lifetimes, and full table loads followed by Rust filtering instead of SQL.
- Output a Markdown table with exact columns: Severity (Blocker / High / Nit),
  File:Line, Anti-Pattern, Concrete Fix. Validate findings against current code.

Use Fish/RTK and Task entry points. Apply TDD to executable changes.
No broad lint suppressions, including tests or generated entities.
Keep Rails read-only; no legacy data/credential import, parity or reverse migration.
Never reset or silently adopt a populated database. Preserve API/MCP contracts
except explicit access/location changes documented in OpenAPI, clients and ADRs.
