## 0. Agent-orchestrated delivery

- [x] 0.1 This coordinating agent performs the implementation preflight: review the complete task list for conflicts, record the initial branch state, create the durable progress ledger, and keep architecture, stack lineage, integration, and final verification in the parent judgment loop.
- [ ] 0.2 Before each implementation task, create a task-specific brief with owned files, interfaces, constraints, acceptance criteria, exact verification commands, risks, and escalation conditions; select the worker model through adaptive routing rather than inherited defaults.
- [ ] 0.3 Dispatch one fresh implementer at a time, require its durable report and self-review, generate a review package from the recorded task base, and obtain an independent task review before dispatching subsequent implementation work.
- [x] 0.4 Route bounded multi-file work and routine reviews to Terra at high effort; reserve Sol at high or xhigh effort for cross-repository integration, concurrency, deployment control, destructive reset behavior, stack changes, unresolved evidence, and final whole-branch review.
- [ ] 0.5 Escalate any broadened scope, public-contract change, data-safety decision, unresolved ambiguity, or second failed evidence-based attempt to this coordinating agent; fix critical or important review findings and re-review before recording task completion.
- [x] 0.6 Keep this MedTracker OpenSpec change and progress ledger as the single planning authority, explicitly cross into `damacus/home-ops` for production-deployment tasks, and load and enforce the owning repository's local instructions before any task changes its code.

## 1. Preserve the landed DM+D application contract

- [x] 1.1 Correlate the production active-import UI with CSP evidence and the shared Puma/Solid Queue process with the OOM-killed import and stranded active records.
- [x] 1.2 Add failing request, broadcast, importer SQL-observation, concurrency, and stale-reconciliation coverage for every MedTracker behavior in the capability spec.
- [x] 1.3 Implement signed live refreshes, uncached archive importing, the one-active-import database invariant, friendly concurrent submission handling, and five-minute stale reconciliation with a 30-minute threshold.
- [x] 1.4 Run focused tests, `task rubocop`, `task test`, desktop/mobile UI verification, and merge green MedTracker PR #1784.
- [x] 1.5 Record merged portable-storage PR #1782 as the durable archive prerequisite, retain issue #1775 for deployment-dependent canary evidence, and do not restack the historical pre-squash storage commit.

## 2. Build the MedTracker stack above the merged baseline

- [ ] 2.1 Re-review MedTracker PR #1785 against current `main`, verify its S3 reset safety and removal of the stopped-web health dependency, and keep its full CI green.
- [x] 2.2 Create the M2 integration branch from the verified PR #1785 tip and move only this OpenSpec change's artifacts onto it; confirm the focused diff contains no duplicate application implementation and no home-ops filesystem or manifest paths.
- [x] 2.3 Run `task openspec:validate`, `task docs:build`, and `git diff --check`, then open M2 as a dependent draft PR whose base is the PR #1785 branch and whose description records the full MedTracker and home-ops stack order.
- [ ] 2.4 After PR #1785 lands, replay only M2 onto current MedTracker `main`, rerun its checks, verify the focused diff, retarget it to `main`, and merge it only with explicit authorization.

## 3. Align the lower home-ops boundary with the application reset contract

- [ ] 3.1 Update home-ops PR #3995 from current upstream `main` and record its focused scope: production-data-free canary rebuild, S3-backed application reset contract, removal of obsolete shared-filesystem assumptions, and no worker-topology changes.
- [ ] 3.2 Add or update failing deployment assertions for exact application image identity, canary-only S3 targets, reset-controller isolation, production-data exclusion, and continued suspension before changing deployment configuration.
- [ ] 3.3 Update PR #3995 to consume the verified PR #1785 image and behavior, make its focused assertions green, and preserve the suspended, non-reconciled release state.
- [ ] 3.4 Run the home-ops focused topology checks, YAML validation, full render/policy checks, and whitespace checks; review the diff against `main` and keep PR #3995 green and review-ready.
- [ ] 3.5 Prove in the lower deployment boundary that removing canary database recovery, WAL, and backup state does not remove or misidentify the selected application blob-storage mode or durable DM+D archive contract; keep canary suspended until this evidence is green.

## 4. Restack worker isolation above the canary rebuild

- [ ] 4.1 Rebuild home-ops PR #3999 from the verified PR #3995 tip, retarget its base to the PR #3995 branch, and resolve overlapping canary configuration in favor of both the demo-reset and worker contracts.
- [ ] 4.2 Add or update failing assertions proving production and canary web processes do not run Solid Queue, workers use `bin/jobs`, worker and web share the exact release configuration and durable storage contract, and each worker has an independent one GiB memory limit.
- [ ] 4.3 Make the H2 topology assertions green without adding demo-reset behavior to the worker boundary or changing the lower canary database and reset contract.
- [ ] 4.4 Run focused topology checks, YAML validation, full render/policy checks, and whitespace checks on the cumulative branch; review H2 against H1, document stack order and deferred activation, and keep PR #3999 draft until PR #3995 is accepted.

## 5. Land lower first and validate canary

- [ ] 5.1 With explicit merge authorization, merge MedTracker M1 before M2 and home-ops H1 before H2; after every lower merge, replay and reverify only the remaining upper boundary against current `main`.
- [ ] 5.2 Publish or resolve the exact cumulative MedTracker image, confirm both canary deployment boundaries are green, and obtain separate authorization before reconciling the suspended canary.
- [ ] 5.3 Import a full public DM+D release in canary and capture PHI-safe evidence that progress updates without reload, the worker stays within one GiB, the web process remains healthy without restarts, the import completes, and final counts and log match persisted data.
- [ ] 5.4 Complete issue #1775's applicable canary storage/restore, archive, and background-job evidence using the cumulative image and deployment contract; record only PHI-safe results.
- [ ] 5.5 Exercise the weekly canary reset around ordinary web and worker execution, verify the S3 cleanup and synthetic baseline remain correct, and correct any failure in its owning lower boundary before restacking upward.
- [ ] 5.6 Collect synthetic medication-administration persistence, reminder-event, and delivery evidence with PHI-safe logs and traces; retain this as a separately owned follow-up if the upstream persistence question remains unresolved rather than attributing it to DM+D import behavior.

## 6. Promote and close the incident

- [ ] 6.1 Promote the identical tested application image and worker contract to production only after every canary acceptance invariant passes.
- [ ] 6.2 Run and observe a production DM+D import, verify live progress, worker memory, web health, terminal counts, and PHI-safe logs, and confirm interrupted historical imports are shown as failed rather than active.
- [ ] 6.3 Record acceptance evidence and final PR/image identities in the originating incident context, update this OpenSpec task list and both repositories' PR records to match landed reality, and archive this change only after no deployment or operational acceptance work remains.
