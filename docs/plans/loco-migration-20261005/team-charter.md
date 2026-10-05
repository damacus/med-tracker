# Loco migration team charter

This charter supersedes older Axum/Leptos port allocations for this migration.
Apply the user's team-development skill and repository instructions.

The user's retrospective correction supersedes earlier task-level ceremony.
Locally verified dependencies permit isolated implementation; hosted CI is an
acceptance/merge gate, not an implementation queue. The coordinator independently
reviews routine tooling/CI repairs with focused tests. Devin reviews coherent
capability changes and security/schema decisions, rather than every tiny repair.
Keep one writer on shared persistence until its interfaces are stable. Preserve
all correctness, interoperability, rollback and final release requirements.
Start each new capability review in a fresh Devin session with its frozen source
and acceptance evidence. Resume only to address findings within that capability.
For unattended source reviews, supply the required source in the prompt and avoid
tool calls that need interactive approval; never broaden tool permissions to
work around a stalled review.

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

## Retrospective handoff and implementation ownership

The user switched to Astra after the CI repair reached a clean published boundary
at `705e1f09`. The previous Sol writer has finished that work and is idle. Astra
now owns the sole writer role for P2/P3 persistence, including migration, fixture
runner changes, entities and shared access/error interfaces. This is a recorded
handoff at a new capability boundary, not concurrent editing of the same source.
The existing verifier remains the only owner of builds and database resources.

Once the shared tenant/actor/error interfaces pass focused checks, the Sol writer
may separately own care-operation modules and their tests. It must port existing
SeaORM transaction/locking/audit logic and request shared-interface changes from
Astra. Give each writer an explicit disjoint file set before dispatch. Add browser
ownership only after the relevant domain operation is stable. The coordinator
retains integration and Devin retains coherent capability/security review.

This two-owner arrangement supersedes the earlier global prohibition on parallel
product writers only after those interface and ownership prerequisites are met.
Until then Astra is the sole product/test writer. All later references to the
same writer mean the owner of that capability; review fixes return to that owner.
When a writer has completed its turn, dispatch a follow-up task for fixes; a queued
message alone does not restart an idle writer. Keep the verifier's lane moving.

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

Outside the explicit disjoint ownership arrangement above, no parallel writers
may edit shared product or test files. The owning writer fixes findings and receives
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
that passing input. Source-only review and the whole-suite check may run in
parallel against that same freeze: neither may modify it. Publication requires
both to pass. If review requires executable changes, freeze the correction and
verify the resulting source again; old evidence cannot certify changed inputs.
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
