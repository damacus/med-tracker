# Nightingale foundation brief

Read plan.md, team-charter.md and repository instructions first. Own all product
and test edits for tranche 1, including root layout, manifests, tasks and CI/path
contracts. Preserve existing source/comments, behavior and unrelated work. Root
owns planning/charter/progress; you own foundation-report.md and inventory records.

Deliver a real standard Loco root application with an observable health route,
Tera initialization and no wrapper around the old Axum router. Use pinned current
Loco, PostgreSQL18 safe config (no implicit migration/truncate/recreate). Relocate
Rails runtime under rails/ and make all its root entry commands rails: only;
keep it independently runnable. Inspect all build paths/CI/scripts/generators
before moving; retain shared root OpenAPI/native/docs paths and client contracts.
Old Rust sources may remain as explicit migration inputs until their tranche,
but cannot be the new application's router or root task implementation.

First create failing meaningful layout/task/routing tests; ask the exclusive
verifier to record RED before implementation. All runtime/build/install commands
go to that verifier via messages. Task listing/reads are okay. Declare the exact
Task commands needed; add prerequisite test task wiring without product code if
needed to execute RED. No direct Cargo/Docker/Rails builds outside Task commands.

Provide source-derived capability/route/job/theme/security-format inventory with
revision and concrete source pointers, plus PR salvage/disposition records. Pin
stable official Loco docs/source APIs; Context7 may lag upstream. Verify worker
and SMART/FHIR dependency feasibility; do not implement sensitive protocol code
in this tranche. Report an actual unsupported prerequisite immediately.

Acceptance: root Loco compile/tests/health/Tera smoke, root command ownership,
safe configs, Rails task/CI relocation path contracts and rollback boot smoke,
machine-checkable inventory drift where practical, independent Devin review.
Do not weaken old tests or delete functionality to make the foundation pass.
Update relevant docs/tooling and AGENTS.md/agents.md together for new layout.

No Git actions, PR mutations, deployments, data resets or parallel writers.
Escalate ambiguous schema/auth/session decisions to Bucky. Return compact status
and a detailed report with exact checks, red/green evidence, changed paths,
unverified claims and next safe action. Stay available for review fixes.
