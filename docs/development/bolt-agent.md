# Bolt: Rust/Loco performance agent

Bolt investigates a real bottleneck, then implements one small measured
improvement only after the owner approves its concrete scope. The initial
performance chat is investigation-only. Adding the agent does not approve an
application change, a dependency, a heavy build or a load test on shared resources.

## Configuration and starting a fresh chat

[`.codex/agents/bolt.toml`](../../.codex/agents/bolt.toml) defines the project
custom agent named **`bolt`**, using `gpt-6.1-sol` with explicit `medium` reasoning.
It uses the standalone TOML format in the
[official app custom-agent documentation](https://learn.chatgpt.com/docs/agent-configuration/subagents?surface=app#custom-agents).
No global config, permission policy, provider or MCP server is overridden.
The instructions preserve repository architecture, correctness and delivery rules.
They reuse the existing architecture, tooling, drift-audit and testing skills.

Start the new chat in a checkout containing the reviewed agent file. A fresh
checkout of `main` will not contain an unmerged addition. Ask the new chat to use
the project custom agent `bolt`, for example:

> Use the project custom agent bolt to investigate one Rust/Loco performance
> bottleneck. Investigation only: trace the actual path, assess existing evidence,
> propose a reproducible measurement and return one concrete scope for approval.
> Coordinate shared resources before any heavy build or load test. Do not edit
> application code, add dependencies, publish, merge or deploy.

Custom agents are subagent types, not a separate top-level chat model. The new
chat selects `bolt` when delegating the investigation. The parent retains its
permission settings. No new chat or performance job is started by this setup.
If the client cannot select the custom type, report that limitation rather than
silently substituting a generic worker.

Validation uses standard TOML parsing and Codex's local prompt renderer to check
discovery. This confirms configuration loading, not completion of an investigation
or a performance improvement. The installed renderer's command is:

```fish
rtk proxy codex -C /path/to/checkout debug prompt-input 'Validate configuration rendering only.'
```

Its output can contain local instructions: inspect it locally, do not publish
the full rendered prompt. This command renders context without requesting a model
turn. No application build is needed for this configuration/documentation change.

## External skill assessment

Reviewed on 23 October 2026. **No assessed skills or profiling tools were installed.**
The linked instructions and selected relevant references were read as source
material, not executed. All four repositories' actual `LICENSE` files declare
MIT. Dates below are repository revision dates, not a maintenance guarantee.
Popularity and catalogue audit badges were not treated as evidence of safety.

| Skill/source | Revision reviewed | Recommendation and reason |
|---|---|---|
| [Apollo rust-best-practices](https://github.com/apollographql/skills/blob/f9843dce86b1a561e3bfbb024e65c28a6cbbe38c/skills/rust-best-practices/SKILL.md) | `f9843dce86b1` · 20 Oct 2026 | Supplementary reading, not another default policy. Its performance chapter supports release profiling and avoiding unneeded allocations. Existing project skills already govern linting, tests and errors; do not import generic size/percentage thresholds, automatic toolchain updates, dependency updates or lint exceptions. |
| [Requested rust-security](https://github.com/mohitmishra786/low-level-dev-skills/blob/bdc58472fa9f309ed1b3f7d985a0d8e9bd8f4608/skills/rust/rust-security/SKILL.md) | `bdc58472fa9f` · 27 Jun 2026 | Avoid installing unchanged. Its audit/Miri/fuzzing topics are useful, but the FFI example's null/length check does not establish allocation validity, lifetime and aliasing requirements. Its cargo-deny example contains obsolete policy fields and an example advisory ignore. Use current maintainer documentation and the existing security workflow instead. |
| [rust-profiling](https://github.com/mohitmishra786/low-level-dev-skills/blob/bdc58472fa9f309ed1b3f7d985a0d8e9bd8f4608/skills/rust/rust-profiling/SKILL.md) | `bdc58472fa9f` · 27 Jun 2026 | Best focused candidate for a later project adaptation: release symbols, flamegraphs, allocation and Criterion measurements. Do not install unchanged: it includes tool installation, privileged profiling, kernel-setting/ASLR changes and example manifest edits. First prove which tool the investigation needs. |
| [linux-perf](https://github.com/mohitmishra786/low-level-dev-skills/blob/bdc58472fa9f309ed1b3f7d985a0d8e9bd8f4608/skills/profilers/linux-perf/SKILL.md) | `bdc58472fa9f` · 27 Jun 2026 | Reference only for an approved Linux capture. Persistent sysctl examples, system-wide collection and simplified counter thresholds are unsuitable defaults on this shared macOS/Docker host. Do not lower isolation or install packages automatically. |
| [Supabase Postgres best practices](https://github.com/supabase/agent-skills/blob/c9be0e931b7930f7d02126d04774d904c381e7d7/skills/supabase-postgres-best-practices/SKILL.md) | `c9be0e931b79` · 2 Oct 2026 | Useful targeted query-plan, pooling, batching and RLS references; consider a scoped adaptation if database evidence warrants it. Do not import Supabase-specific auth functions, raw-SQL application patterns, blanket index rules or advertised multipliers. A sequential scan can be the correct plan; PostgreSQL 18 and SeaORM/RLS remain authoritative here. |
| [rust-async-patterns](https://github.com/wshobson/agents/blob/0874a27b17a22e2b7c2e2bcd725768e0803cad06/plugins/systems-programming/skills/rust-async-patterns/SKILL.md) | `0874a27b17a2` · 17 Oct 2026 | Selected conceptual reading for bounded tasks, cancellation and tracing, not wholesale installation. The detailed reference includes a custom pool whose `Drop` spawns a task with a borrowed pool; do not copy that lifecycle pattern or its `unwrap` examples into Loco. Use framework-owned pools and resources. |

The two requested catalogue entries are
[Apollo](https://www.skills.sh/apollographql/skills/rust-best-practices) and
[rust-security](https://www.skills.sh/mohitmishra786/low-level-dev-skills/rust-security).
The table links the actual reviewed sources rather than an unpinned installer.

## Prefer primary references for the selected measurement

- [Rust Performance Book: profiling](https://nnethercote.github.io/perf-book/profiling.html)
  explains selecting a profiler for the actual bottleneck.
  [samply](https://github.com/mstange/samply) is a useful macOS/Linux option;
  availability and permission requirements must be checked before use.
- [Tokio console](https://github.com/tokio-rs/console) helps investigate async
  task behaviour. Its instrumentation, dependencies, runtime configuration,
  overhead and diagnostic endpoint exposure need assessment before adoption.
- [PostgreSQL 18 EXPLAIN](https://www.postgresql.org/docs/18/using-explain.html)
  is the source for interpreting plans; `ANALYZE` executes the statement.
  Use authorised disposable fixtures, real tenant roles and repository-approved
  diagnostic access, not production data or a SeaORM bypass.
- [Rust slice safety](https://doc.rust-lang.org/std/slice/fn.from_raw_parts.html)
  states the full raw-pointer contract omitted by the security example.
  [Current cargo-deny configuration](https://embarkstudios.github.io/cargo-deny/checks/advisories/cfg.html)
  should replace copied policy snippets, never automatically weaken checks.

Context7 was attempted during this assessment but its quota was exhausted;
maintainer documentation supplied the fallback. This is a source assessment,
not a security certification of the repositories or every referenced file.
