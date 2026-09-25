# Axum and Leptos port ledger

Plan: `docs/plans/axum-leptos-port/plan.md`

Planning baseline: `781c3a297ffe24ae5b70f893d55b568eab1216fc`.

## Decisions

- Observable behaviour parity replaces one-for-one RSpec translation.
- Existing accounts and medical data survive cutover; reauthentication is
  acceptable.
- Combined application-process target is under 200 MB; PostgreSQL and
  supporting services are excluded.
- API tests precede broad UI parity tests; product implementation follows both
  baselines except the bounded Leptos foundation ruling below.

Ruling: start one isolated Leptos SSR login-page foundation before the full
API and browser baselines close — the user explicitly requested an early UI
start with an expected-red Rust smoke — the cost if wrong is reworking the
shell when shared API, router, and authentication interfaces are settled.
This does not count as UI, PWA, or workflow parity.

Ruling: port the existing login-form display example in
`spec/system/user_sessions_spec.rb` and the public heading/OIDC-absence
example in `spec/features/security/oidc_security_spec.rb` as the foundation
smoke — the Rails suite is the behaviour source, and the user rejected an
invented substitute — the cost if wrong is later reworking this small smoke
when the full authenticated login journey is ported.

| Owner | Exclusive paths | Reserved shared paths |
| --- | --- | --- |
| API contract writers | Their separate worktrees; `rust/contract-tests/**`, `rust/parity-matrix.md`, API fixture and contract-task changes | No `rust/web/**` edits |
| Leptos foundation writer | Separate worktree and branch; `rust/web/**` only | No root workspace/lockfile, shared router, database/schema, API fixtures, parity matrix, or contract runner edits |
| Coordinator | Plan, ledger, serial integration and combined checks | Root workspace/router/API integration after UI review |

## Progress

- Tranche 0: API inventory and disposable contract runner accepted through
  reviewed API-plan Tasks 1 and 2; the matrix records all 118 OpenAPI
  operations and the existing RSpec inventory.
- Tranche 1: in progress. Reviewed API contract slices through web JSON reads,
  web JSON actions, and cross-account session denial are integrated. The
  complete combined Rails run last had one FHIR fixture-state failure; the
  corrected focused FHIR suite passed 5/5, and combined rerun is pending.
  Provider delivery, production-like CSRF, and known Rails privacy/image
  defects remain explicit gaps.
- Tranche 2: full browser/PWA parity pending. The bounded Leptos login-page
  foundation is in progress in its isolated worktree; it does not close this
  tranche.
- Tranche 3: pending.
- Tranche 4: pending.
- Tranche 5: pending.

Record each accepted task as `Task N: complete (commits BASE..HEAD, review
clean)` and each unresolved decision as `Ruling: decision — evidence — cost if
wrong`. Do not mark a task complete from a test run without its independent
review and recorded result.
