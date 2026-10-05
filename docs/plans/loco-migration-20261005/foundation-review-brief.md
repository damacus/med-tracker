# Independent foundation review brief

Reviewer: Devin CLI, explicit `swe-2-max`. Review is read-only. The user approved
Devin plan review; separate implementation-source transfer approval is pending.
Do not dispatch this brief until that approval arrives. Repository
is public: <https://github.com/damacus/med-tracker>. Do not read environment files,
credentials, machine key material, user session/config files or unrelated trees.
No builds, installs, tests, mutations, Git writes, PR actions or extra agents.

Review the actual combined foundation diff against
`838e79da76aceed1155d047dae038e1bd2bad5f1`, with rename detection. Read plan.md,
team-charter.md, foundation-brief.md, foundation-report.md and the verifier evidence
supplied at dispatch. New planning prose is Bucky-owned; application, tests,
relocation and tooling are the sole Nightingale writer's work.

Give separate requirements and technical-quality verdicts. Identify Critical and
Important findings with concrete file/line evidence and a minimal remedy. Resolve
requirements based on this tranche: a real root Loco boot/routing/Tera foundation,
intact independent Rails relocation, namespaced Rails command ownership, CI and
contract-runner path correctness, safe DB configuration, inventory/PR preservation.
Do not treat later migration tranches as silently complete or demand their entire
feature implementation from the foundation.

Pay particular attention to Ruby setup/working-directory/cache paths, Docker build
contexts, RSpec/screenshot/fixture paths, root authoritative OpenAPI and client
generation, old contract harness calls, readiness/liveness, no old Axum wrapper,
listener/process cleanup, temporary data ownership, source comments unchanged,
new path classification and CI failure propagation. Check generated-file lint
exclusions after Rails container Git-root changes, and the stored preservation
ledger's symlink targets and executable modes. The staged AGENTS.md and agents.md
must be identical even on a case-insensitive host filesystem.

Inspect owning code before accepting claims. Distinguish an untested concern from
a verified finding; do not assume Devise or Leptos offline behavior. Official Loco
1.2 supports PostgreSQL queues; critique specific reliability contracts rather than
assuming another framework is necessary. Output review to stdout only. Bucky will
retain it as a review artifact and route fixes to the same writer.
