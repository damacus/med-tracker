# behaviour-tests owner brief

Model/effort: Sol 6.1 High.

Owned paths: rust/web/tests/household_completion_i18n.rs; rust/web/tests/household-completion-locales.test.mjs; rust/web/tests/household-completion-medication.test.mjs; rust/web/tests/household-completion-dashboard.test.mjs; rust/contract-tests/tests/household_completion_locales.rs; rust/contract-tests/tests/household_completion_medication.rs; rust/contract-tests/tests/household_completion_dashboard.rs.

Separate test writer for A/B/C. Queue smallest observable RED against unchanged product, then release behaviour to owner. Design deterministic medication interleaving with real HTTP persistence, not timing sleeps. Agree any fixture changes/helper transfer with coordinator; fixture.rs remains unowned until explicit transfer. New files only by default; do not modify old expectations/selectors to pass. Prioritise B/C RED then A locale failures; keep exact values, stock/no-write/read-back and health-data isolation.

## Shared rules

Read AGENTS.md, the saved execution packet, team charter and relevant Rust skill
references. Fish + RTK + repository Tasks. No source comment additions/removals.
Do not switch branches, commit, push, mutate shared wiring or edit another owner's
paths. Preserve unrelated Rust skill/stashes. Serena first; Rust symbolic support
may be unavailable. Context7 before new library/API usage details.

Only Luna runs compiled/runtime checks, dependencies, shared UI builds, Docker and
fixture/bootstrap. Submit exact jobs; no second build. Product code follows actual
recorded RED for its behaviour. Discover/design before RED; do not implement early.
Report implemented/reviewed/verified separately, with file list, exact evidence,
risks, next action. Two unsuccessful fixes or scope/security ambiguity escalate.
