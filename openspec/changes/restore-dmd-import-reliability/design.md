## Context

See [proposal.md](proposal.md) for the incident and motivation and [the capability specification](specs/dmd-import-reliability/spec.md) for observable behavior.

The application repair in `damacus/med-tracker#1784` is merged. It replaced the production-CSP-blocked reload, bounded importer caching, added the active-import invariant, and scheduled stale-run reconciliation. Portable storage is also merged as `damacus/med-tracker#1782`; its original branch commit was squash-merged, so the current `main` merge commit is the only valid prerequisite identity. The remaining deployment work in `damacus/home-ops#3999` overlaps `damacus/home-ops#3995`, while the related application-side canary reset correction is under review in `damacus/med-tracker#1785`.

The two repositories have different code ownership and cannot form one GitHub branch stack. MedTracker owns import and reset behavior; `damacus/home-ops` contains the process topology and release activation needed to run that behavior in production. This OpenSpec change remains rooted in MedTracker as the single planning authority for the MedTracker production capability, while its explicitly authorized implementation scope crosses the repository boundary. It records deployment requirements without naming or depending on home-ops filesystem or manifest paths.

## Goals / Non-Goals

**Goals:**

- Preserve the merged DM+D reliability contract while the canary reset stream is integrated.
- Control application and production-deployment implementation from this MedTracker-owned change and progress ledger.
- Make the application-to-deployment handoff behavioral, versioned, and testable.
- Use focused same-repository stacks with lower-first review and landing.
- Keep every intermediate branch green and either deployable or explicitly dormant.
- Require a full canary DM+D import before production topology promotion.

**Non-Goals:**

- Create a cross-repository Git stack or copy deployment implementation into MedTracker.
- Move the OpenSpec planning authority into a standalone store or the deployment repository.
- Reopen or rewrite merged PR #1784.
- Reconcile the suspended canary or mutate production as part of proposal work.
- Make the demo reset depend on Solid Queue or make DM+D imports depend on the reset scheduler.

## Decisions

### 1. Preserve the merged application reliability boundary

Use the signed record stream already merged in PR #1784 for active-page refreshes and committed broadcasts for progress and terminal states. Keep the importer inside an uncached Active Record boundary, retain the PostgreSQL partial unique index for active states, and run stale reconciliation every five minutes with a 30-minute threshold.

These are one application contract: live feedback without reliable execution would still strand operators, while reliable execution without live feedback would still reproduce the reported production symptom.

**Rejected:** Restore timed polling with a nonce or relaxed CSP. It adds avoidable traffic and reintroduces script-policy coupling when the application already has a database-backed Turbo Stream transport.

**Rejected:** Enforce active-import exclusivity only in the controller. Concurrent requests and non-HTTP callers require a database invariant.

### 2. Treat the deployment worker as production implementation of the MedTracker capability

The deployment repository provides a dedicated `bin/jobs` worker for production and canary, removes in-web Solid Queue execution, and gives the worker the same image, environment, secrets, and durable storage contract as the web process with a separate one GiB memory limit. This is production code for the MedTracker capability, not an optional external follow-up. MedTracker verifies its importer and reconciliation behavior; the deployment repository verifies rendered topology and release activation.

No MedTracker artifact or implementation shall name home-ops directories, manifest filenames, or YAML structure. Cross-repository references are limited to repository identity, PR identity, image identity, observable process topology, and acceptance evidence.

**Rejected:** Share a pod-wide one GiB limit between Puma and jobs. An importer OOM would continue to threaten web availability and health probes.

**Rejected:** Duplicate deployment YAML expectations in MedTracker. That couples application code to another repository's layout and creates two authorities for topology.

### 3. Integrate the canary reset before restacking worker topology

PR #1785 changes the reset operation to clean the canary-only S3 target directly and removes its impossible application-health dependency while web is stopped. The corresponding lower deployment boundary, PR #3995, must be updated to consume that application contract and remove obsolete shared-filesystem assumptions before PR #3999 is rebuilt above it.

The worker remains independent of the reset controller. The reset quiesces web and queue writers for its destructive window; the worker topology supplies ordinary asynchronous execution outside that window.

**Rejected:** Merge PR #3999 directly into the current PR #3995 diff. Keeping the worker layer separate gives focused ownership for Puma isolation and allows lower canary-reset corrections to propagate upward deliberately.

### 4. Use two repository-local stacks and one release dependency

This MedTracker OpenSpec change is the single planning source for both stacks. The coordinating parent crosses the repository boundary to execute deployment tasks, but each implementation task follows the instructions, validation commands, commit history, and review process of the repository whose code it changes. The repository boundary therefore controls code ownership without splitting release authority.

#### Delivery Shape

**Mode**: Stacked PRs
**Stack scope**: Cross-slice coordination with repository-local lineage
**Repository topology**: Two same-owner repositories; no cross-repository stack
**Reason**: The changes overlap in canary configuration and OpenSpec intent, but each repository requires focused diffs, independent CI, and its own merge history.
**Story scope**: Restore trustworthy DM+D imports and prove them in the production-data-free canary without weakening the weekly reset.
**Done when**: Both stacks are landed, a full canary import passes every acceptance invariant, production is promoted with the identical image and worker contract, and interrupted historical runs are reconciled.

| # | Boundary | Base | Owns | Depends on | Verification | Release state |
| --- | --- | --- | --- | --- | --- | --- |
| P0 | Portable storage, PR #1782 | `main` | Durable, service-neutral application archive contract | - | Landed application and storage smoke evidence | Landed historical prerequisite |
| M0 | MedTracker DM+D reliability, PR #1784 | `main` | Live progress, bounded import, exclusivity, stale reconciliation | - | Full MedTracker CI | Landed |
| M1 | Canary S3 reset, PR #1785 | current MedTracker `main` | Reset safety and S3 cleanup behavior | M0 application baseline | Full MedTracker CI | Review-ready |
| M2 | DM+D OpenSpec integration | PR #1785 branch | This proposal, capability, design, tasks, and combined acceptance contract | M1 | OpenSpec validation, docs build, diff check | Draft until M1 is accepted |
| H1 | Canary demo rebuild, PR #3995 | current home-ops `main` | Production-data-free canary aligned with M1, kept suspended | Published M1 image contract | Focused topology checks and full render/policy CI | Review-ready after alignment |
| H2 | Worker isolation, PR #3999 | target: PR #3995 branch; current: `main` | Production and canary worker sidecars plus topology assertions | H1 configuration contract | Focused topology checks and full render/policy CI | Current: open against `main`; target: dependent draft until H1 is accepted |
| E1 | Portable-storage canary evidence, issue #1775 | cumulative canary state | Storage round trips, restore, archive, and background-job evidence | P0, M1, H1, H2 | PHI-safe canary acceptance evidence | Operational gate |
| O1 | Medication-administration persistence observation | separate evidence lane | Distinguish persisted administrations from reminder events and delivery | Synthetic canary baseline | PHI-safe persisted-data, event, log, and trace evidence | Non-stack follow-up |

P0 and M0 are already on `main`; do not recreate their historical branches or claim that their pre-squash commits remain active parents. Review and land M1 before M2 in MedTracker, and H1 before H2 in home-ops. E1 converges the two stacks at canary acceptance. O1 is deliberately integrated into the release evidence ledger but not forced into a Git stack because the session established no code patch or hard lineage; its unresolved outcome must remain visible and PHI-safe. Changes to a lower boundary are made there and propagated upward; upper branches do not carry workarounds for lower defects. Application images are published before a deployment boundary consumes them. The live cluster remains unchanged until the cumulative branches are green and activation is separately authorized.

### 5. Preserve all active-session evidence without inventing branch lineage

The three active sessions are one release ledger:

- **Portable storage and missed-dose investigation (`019fafbe-9a8f-7291-81e7-4f29d7603dc9`)** established the durable, service-neutral application archive contract now landed as PR #1782 and identified issue #1775 for separately deployed canary evidence. Its historical dependency on PR #1766 is closed because #1766 is merged. The session also found that reminder events matched persisted data but could not prove the upstream administration-persistence path.
- **Canary reset design and purge runbook (`019fb872-27db-7121-828c-8108c2dbb966`)** established M1/H1 as the lower boundaries, confirmed that no destructive purge or replacement deployment ran in that session, and requires the canary to remain suspended until the production-derived recovery and backup configuration is replaced. It distinguishes backup-free CNPG recovery from application blob storage.
- **DM+D integration (`019fc792-51a7-73a3-897d-b997229e24a1`)** established M0 as merged and H2 as the upper worker-isolation boundary, then requires all three session outcomes to converge at canary acceptance.

The ledger does not assert a technical dependency merely because sessions are active together. It records each owner and gate, preserves an unresolved observation, and uses a stacked branch only where M1→M2 or H1→H2 has real lineage.

**Rejected:** Fold the missed-dose observation into the DM+D implementation branch. The session found no causal link or patch. Forcing it into the stack would hide the unresolved persistence question behind unrelated code churn.

**Rejected:** Treat deletion of canary database backup resources as proof that S3-compatible application storage is absent. Database recovery and application blobs have distinct contracts and must be validated independently.

### 6. Make canary acceptance the promotion authority

Canary runs a full public DM+D archive using the exact candidate image and worker topology. Acceptance observes live progress, worker memory, web restarts and probes, terminal state, final counts, log, and persisted data. The public archive contains no medication or household data.

Production promotion reuses the exact tested image and worker configuration. A failed invariant routes correction to its owning lower boundary and triggers restacking and cumulative verification above it.

**Rejected:** Promote after synthetic or partial importer tests alone. The production failure involved CSP, process topology, memory pressure, queue execution, and a full archive; only an end-to-end canary run exercises that combined contract.

### 7. Use adaptive agent orchestration for implementation and review

This coordinating agent is the parent for the change. It retains the Sol-level judgment loop: architecture, authorized cross-repository execution, code ownership, exact stack bases, security and data-safety decisions, integration, final verification, and the user-facing handoff. The MedTracker OpenSpec change and its durable progress ledger remain the control location throughout. Before work in either repository, the parent loads and enforces that repository's local instructions. It prepares one task-specific brief and report location per task, and dispatches no parallel implementation agents because the stack changes overlap.

Route bounded multi-file implementation, test expansion, and routine task review to Terra at high reasoning effort. Use Sol at high or xhigh reasoning effort for cross-cutting design, concurrency, deployment control, destructive reset behavior, branch-lineage changes, unresolved production evidence, and final whole-branch review. Escalate a task back to the coordinating agent when its scope broadens, a public or data-safety contract is exposed, or two evidence-based attempts fail. Model choice is reassessed at every handoff rather than inherited by default.

Every implementer receives only its objective, owned files, relevant interfaces, non-goals, exact acceptance criteria, verification commands, risks, and escalation conditions. After each implementation task, an independent reviewer checks specification compliance and task quality from a review package. Critical or important findings return to a bounded fixer and are re-reviewed before the next task. The parent writes reviewed completion to the ledger; a final Sol review covers the whole cumulative branch.

**Rejected:** Let one agent implement, review, and integrate a task. The DM+D incident crosses concurrency, memory isolation, and two repositories, so unreviewed self-certification is insufficient.

**Rejected:** Run implementation agents in parallel. The canary reset and worker topology overlap, and parallel edits would obscure branch ownership and make stack restacking less safe.

## Risks / Trade-offs

- **[A lower stack change invalidates an upper diff]** → Fix the lower boundary, restack every upper branch, rerun focused and cumulative checks, and re-review immediate-parent diffs.
- **[The canary reset and worker both mutate queue state]** → Keep reset scheduling outside Solid Queue, quiesce ordinary writers during reset, and verify the worker resumes only after reset invariants pass.
- **[A worker OOM leaves an import active]** → Keep web and worker lifecycles isolated and let recurring stale reconciliation record the interrupted failure.
- **[The same image behaves differently between environments]** → Share the release identity and application configuration contract, then verify effective canary and production topology in the deployment repository.
- **[Canary backup removal is conflated with application storage removal]** → Verify the selected application blob-storage mode and durable DM+D archive separately from CNPG recovery, WAL, and backup absence.
- **[The unresolved missed-dose observation is lost while integrating other work]** → Keep O1 in the release ledger, collect synthetic canary persisted-data and notification evidence, and retain a separately owned follow-up if the cause remains unresolved.
- **[Cross-repository details drift]** → Reference behavioral contracts and PRs from MedTracker, never external paths; verify concrete topology only in the repository that owns it.
- **[Stacked CI or review becomes stale after restacking]** → Require current checks and focused parent-relative review before each lower-first merge.
- **[A worker makes a stack or deployment decision outside its brief]** → Escalate to the coordinating agent, update the design and briefs, then resume only affected work.

## Migration Plan

1. Treat merged PRs #1782 and #1784 as immutable baselines; record their merge identities and do not recreate the historical storage branch or rewrite the merged DM+D branch.
2. Finish review of MedTracker PR #1785 and publish its exact application image.
3. Create the MedTracker OpenSpec integration branch from PR #1785, move only this change's artifacts onto it, validate, and open it as the upper dependent PR.
4. Update home-ops PR #3995 to the S3-reset application contract, preserve the distinction between database recovery removal and application blob storage, rerun its focused and full checks, and keep canary suspended.
5. Rebuild home-ops PR #3999 on the verified PR #3995 tip, resolve overlap in favor of the combined demo-reset and worker contracts, retarget it to the lower branch, and rerun cumulative checks.
6. Merge each repository stack lower first. After each lower merge, rebase or replay only the upper boundary onto current `main`, verify its focused diff, and retarget it before merge.
7. Reconcile canary only after the candidate image and cumulative deployment configuration are available. Run the full DM+D acceptance import and issue #1775 storage/restore evidence.
8. Capture synthetic medication-administration persistence, reminder-event, delivery, log, and trace evidence as O1; retain a separately owned follow-up if the original persistence question remains unresolved.
9. Promote the identical image and worker contract to production only after required canary acceptance passes, then verify a production import and automatic reconciliation of old interrupted runs.

For every implementation task, the coordinating agent creates the brief and report, dispatches the selected model, obtains independent task review, records the reviewed result in the progress ledger, and only then dispatches the next task. A final Sol whole-branch review precedes any merge request update or production activation.

Rollback leaves production on its previous deployment configuration. If canary activation fails, suspend it again, retain PHI-safe evidence, correct the owning branch, and repeat the lower-first verification flow.

## Open Questions

None.
