# Loco migration team charter

This charter supersedes older Axum/Leptos port allocations for this migration.
Apply the user's team-development skill and repository instructions.

- Bucky (root) owns interpretation, planning records, scope, acceptance, Git,
  commits, push and PR publication. Bucky does not become a second product writer.
- Nightingale is one persistent GPT-6.1 Sol writer, advertised default low,
  owning product, tests and review fixes within the active tranche.
- Hubble is the user's explicitly chosen Devin CLI `swe-2-max` independent
  reviewer, read-only. Give separate requirements and technical-quality verdicts.
- One GPT-6 Luna medium verifier exclusively owns dependency installs, builds,
  runtime checks and acceptance resources. Writers request jobs and freeze inputs;
  quick static reads and task listing are allowed outside the runtime lane.

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
Devin plan review and its architecture/security details. Automatic approval review
requires a separate approval for implementation-source transfer; that question is
pending and the blocked invocation has not run.

Only mark ledger completion after clean independent task/tranche review and
checks. Do not substitute another reviewer model silently if Devin is unavailable.
