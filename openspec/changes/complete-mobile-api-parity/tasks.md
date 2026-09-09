## 1. Planning and stack

- [x] 1.1 Validate all proposal artifacts with `openspec validate complete-mobile-api-parity --strict`, `task docs:build` and `git diff --check`.
- [x] 1.2 Publish the planning branch against main and verify its GitHub URL; append each later verified PR using `gh stack link`.

## 2. Dose outcome foundations

- [x] 2.1 Red: add storage tests for exact source, household integrity, state/take constraints, unique identity, audit and portable ID; observe failure with `task test TEST_FILE=spec/models/medication_dose_occurrence_spec.rb`.
- [x] 2.2 Green/Refactor: implement the dormant occurrence storage and purge inventory from `record-scheduled-dose-outcomes` tasks 1.1–1.2; verify the model and database constraint specs pass.
- [x] 2.3 Red-Green-Refactor: implement formal projection and not-taken resolution from existing tasks 2–3, testing bounded applicability, pauses, PRN exclusion, future refusal, replay and concurrent outcomes through focused service specs.
- [x] 2.4 Red-Green-Refactor: implement canonical take linkage from existing task 4, testing single decrement, mismatch rollback, legacy allocation and immutable takes.
- [x] 2.5 Red-Green-Refactor: integrate dashboard, reminders, history, reports and insights from existing tasks 5–7I, verifying resolved occurrences stop escalation and remain correctly classified.

## 3. Dose outcome API and routine sources

- [x] 3.1 Red-Green-Refactor: implement bounded occurrence reads and not-taken/reopen/take adapters from existing tasks 8–9C, covering every mobile-dose-outcome-api scenario and the relevant admin/clinician/self/carer/parent/unauthorized cases with focused request specs.
- [x] 3.2 Red-Green-Refactor: implement existing scheduled outcome tasks 10–15B for contracts, take sync linkage, portable-v2 round trip, web outcome/correction actions, sync reads and batch mutations; verify focused tests, contract generation and desktop/mobile browser evidence for visible changes.
- [ ] 3.3 Red-Green-Refactor: complete `extend-dose-outcomes-to-routine-assignments` in its declared order, verifying direct routine cycles and exclusion of as-needed sources across API, sync, portable data and existing consumers.
- [ ] 3.4 Verify and publish each dose layer with `task rubocop`, focused changed-area specs, applicable generated-client checks and current GitHub PR metadata; mark corresponding existing-plan tasks only when proven complete.

## 4. Stock removal API

- [ ] 4.1 Red: specify removal success, decimal validation, source access, insufficient stock, replay and rollback plus bounded history in request specs; observe failure through `task test TEST_FILE=<stock removal request spec>`.
- [ ] 4.2 Green/Refactor: add shared-service API adapters, typed history and capability/OpenAPI entries; verify request specs and client contract checks pass.
- [ ] 4.3 Run applicable focused gates and publish the stock layer above the last dose layer; verify GitHub base/head and stack membership.

## 5. Location management API

- [ ] 5.1 Red: specify create/edit/delete, ETag conflicts, retained history, portable IDs, cross-household and role access in location request specs; observe focused failures.
- [ ] 5.2 Green/Refactor: add location writes using shared policies and model guards; verify all location scenarios through focused specs.
- [ ] 5.3 Red-Green-Refactor: add authorized person membership create/delete with duplicate replay and access tests, and verify portable IDs and household constraints.
- [ ] 5.4 Add typed contracts/capabilities, run relevant client checks, `task rubocop` and focused Rails specs, then publish and verify the location stack layer.

## 6. Review and report API

- [ ] 6.1 Red-Green-Refactor: expose filtered paginated review reads and audited ETag-protected review updates, verifying evidence immutability and person-policy scenarios.
- [ ] 6.2 Red-Green-Refactor: share health-history and medicine-review projections for typed JSON and protected PDF, verifying access, date bounds, download audit and renderer failures.
- [ ] 6.3 Publish typed contracts after client checks and focused Rails/Ruby gates; verify the review/report PR targets the location layer.

## 7. Profile and invitation API

- [ ] 7.1 Red-Green-Refactor: add atomic current-profile updates for permitted fields with invalid-update rollback and forbidden-identity-field request tests.
- [ ] 7.2 Red-Green-Refactor: add protected current-person avatar upload/read/removal; verify image validation, revoked access and attachment failure handling.
- [ ] 7.3 Red-Green-Refactor: add authenticated invitation acceptance using the existing acceptance transaction, verifying matching verified identity, replay, expiry, revocation and grant isolation.
- [ ] 7.4 Red-Green-Refactor: add fresh-authorized resend through the existing delivery workflow, testing old-token invalidation and denied stale MFA.
- [ ] 7.5 Add typed contracts and capability metadata, run client and focused Rails/Ruby gates, then publish and verify the profile/invitation stack layer.

## 8. Offline write parity

- [ ] 8.1 Red: specify the advertised operation matrix and explicit online-only rejection with failing sync request tests.
- [ ] 8.2 Red-Green-Refactor: extend replay for care records and review changes through the same services, verifying ETags, current access and retained history.
- [ ] 8.3 Red-Green-Refactor: extend stock/inventory/order/removal and pause/resume/reorder replay, verifying duplicate prevention and same online domain validation.
- [ ] 8.4 Red-Green-Refactor: prove multi-operation rollback including audit/feed state, deterministic responses and revocation after queueing with focused batch integration specs.
- [ ] 8.5 Update capability/OpenAPI/generated contracts, run relevant client checks and focused Rails/Ruby gates, then publish and verify the final implementation stack layer.

## 9. Final verification and handoff

Use focused checks per layer. Run the full Rails suite only for a justified integration gate or a regression that needs it, as requested on 2026-09-08.

- [ ] 9.1 Reconcile every capability scenario and existing dose-plan task against delivered tests and PRs; leave #2152 open until all requested work is delivered.
- [ ] 9.2 Verify all stack bases, pushed SHAs and applicable GitHub checks; record any unresolved blocker in #2152 and provide the stack URL without merging or deploying.
