# Loco migration team charter

This charter supersedes older Axum/Leptos port allocations for this migration.
Apply the user's team-development skill and repository instructions.

Use **slice** for delivery boundaries in the revised implementation plans.
The user requested HTML progress at each completed slice: the coordinator maintains
`/private/tmp/medtracker-migration-status.html` and its requested Markdown status copy.
Include shipped commit, passed checks, actual blocker and next action. Update material
blockers promptly; avoid duplicate narrative reports for each verification attempt.
Write every visible update and team message in plain English, with proper spaces
between words. Explain what is finished, what is being checked, what is wrong and
what happens next. Keep hashes, job handles and detailed commands in the technical
checkpoint or an optional evidence section. Do not present library experiments as
completed application features.
Migration defects stay in the owning slice. Open follow-up issues only for independent
work outside its completion boundary. Preserve existing incident #2453 without
turning every repair into a new issue.

- Bucky (root) owns interpretation, planning records, scope, acceptance, Git,
  commits, push and PR publication. Bucky does not become a second product writer.
- Nightingale is one persistent GPT-6.1 Sol writer, advertised default low,
  owning product, tests and review fixes within the active tranche.
- Hubble is the user's explicitly chosen Devin CLI `swe-2-max` independent
  reviewer, read-only. Give separate requirements and technical-quality verdicts.
- One GPT-6 Luna medium verifier exclusively owns dependency installs, builds,
  runtime checks and acceptance resources. Writers request jobs and freeze inputs;
  quick static reads and task listing are allowed outside the runtime lane.

Use existing public Tasks for verification resources with an explicit
`CONTRACT_PROJECT` command-line assignment. Before starting a diagnostic, inspect
its dry expansion: every Compose project and lock must name the intended
disposable project. Do not wrap included internal Tasks with task-local project
variables; that scope failed to reach the dependency during this tranche.
Preserve pre-existing volumes. Capture resource provenance and retain diagnostic
logs before cleanup; a silent successful migration log cannot prove no database
change occurred.

The verifier may generate root Cargo.lock through the writer-supplied dependency
resolution Task as deterministic install output. Nightingale retains manifest and
lockfile ownership/review; this exception permits no manual source edits. Record
the generated lock identity before subsequent frozen acceptance runs.
The verifier may also run the writer-requested formatter and deterministic
inventory/preservation Tasks. These are generation exceptions, not shared manual
source ownership; report the resulting input identities before acceptance.

No parallel product/test writers. Same writer fixes all findings and receives
re-review. After two failed evidence-based fixes Bucky diagnoses without replacing
the writer or broadening scope. Use durable briefs/reports/ledger. Reports certify
exact immutable inputs. Only Bucky commits or pushes. No live deployment/migration,
merge, destructive data reset or unrequested external messages. The user approved
Devin plan review and its architecture/security details. The user subsequently
approved sending implementation diffs to Devin through 16 October 2026. Apply
that authorization to migration reviews until its expiry; it does not authorize
deployment, merge or unrelated data transfer.

For a new test or a review repair, finish the relevant static checks and focused
behavioural test before sending the final source for independent review. Freeze
that passing input, obtain review, and then run the expensive whole-suite check.
If a whole-suite failure requires diagnosis, test one evidence-based hypothesis
at a time. Separate helper failures from application failures. Retain an unchanged
owned test image for source-only retries, and clean it when the diagnosis ends.

The user redirected this migration away from legacy Rails test repair. For Loco
changes, run the Loco checks and the migrated workflows. Do not expand RSpec
diagnostics or change Rails application behaviour unless a migration defect is
demonstrated or Rails work is explicitly in scope. Preserve the original Rails
tests and rollback evidence. Required hosted checks and final populated rollback
verification remain acceptance criteria.

Only mark ledger completion after clean independent task/tranche review and
checks. Do not substitute another reviewer model silently if Devin is unavailable.
