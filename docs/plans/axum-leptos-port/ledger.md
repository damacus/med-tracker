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
- Persistence remains undecided until typed, parameterized SQL and an ORM are
  compared against representative PostgreSQL 18 operations and memory data.

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

Ruling: the portable Chromium smoke excludes OIDC/SSO provider buttons while
allowing `Continue with Passkey` — Rails reveals that separate passkey control
in a real browser although the existing no-OIDC system spec uses a broad
`Continue with` matcher under its test setup — the cost if wrong is missing a
newly named OIDC provider button until the full authentication browser suite.

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
  web JSON actions, cross-account session denial, platform/web profile CSRF,
  web JSON action CSRF, and push delivery are integrated. The full isolated
  Rails contract corpus passed with 227 tests, 0 failures, and 13 ignored
  cases after the FHIR fixture and shared-stock expectations were corrected.
  The feature-disabled AI case also passed in its separate target. Known
  Rails privacy, clinical-input, and image-representation defects remain
  explicit gaps.
  The sync change-feed privacy regression is integrated: focused Rails and
  Rails-backed Rust contract checks passed, as did the isolated full Rails
  suite (6,102 examples, 0 failures). The integrated privacy contract passed
  1/1. Legacy ownerless rows and deleted-Person tombstones remain manager-only;
  restricted clients recover by replacing local state from a full snapshot.
- Tranche 2: full browser/PWA parity pending. The bounded Leptos login-page
  foundation is integrated and passed its desktop/mobile Rust browser smoke;
  it does not close this tranche or prove authentication, PWA, or visual parity.
- Tranche 3: pending.
- Tranche 4: pending.
- Tranche 5: pending.

Record each accepted task as `Task N: complete (commits BASE..HEAD, review
clean)` and each unresolved decision as `Ruling: decision — evidence — cost if
wrong`. Do not mark a task complete from a test run without its independent
review and recorded result.

Accepted API slices: platform/web profile CSRF (`aaf11f35`, independent review
clean); push delivery (`fd82ca4f`, independent review clean); web JSON action
CSRF (`8b96cda8`, independent review clean). Push delivery's combined-run
target list was corrected after the fake runner first failed; the full
isolation and failure-aggregation test then passed. The full Rails corpus
first failed because the web scan test expected the fixture's original stock
after earlier dose tests mutated it. The corrected test (`f9a8bf0c`) compares
the web scan with an authorised live API read, allowing only the two endpoints'
documented decimal display difference; focused Rails passed 7/7 and the full
corpus passed 227/0/13. These are contract tests against Rails, not a Rust API
implementation.

Bounded Leptos foundation Task 1: complete (`b14a8351..a86710ab`, independent
review found no actionable issue after checking the report and four
screenshots). Rails and Rust browser smokes each passed 2/2 across desktop and
mobile. Integrated Rust unit, formatting, lint, and browser smoke passed; the
browser smoke passed 2/2 after the pinned Playwright dependencies were
installed with `10fa0129`. The corrected portable browser smoke did not have
a strict failing-test-before-production sequence, which the writer reported.
The visible `Forgot?` link currently leads to an unimplemented reset route;
that journey and visual parity remain in the later browser tranche.
Fresh integrated screenshots are saved under
`docs/screenshots/leptos-foundation/` (`840fd64b`).

Sync privacy slice of Task 5B: complete (`20348a31`, independent review found no
remaining findings). The change feed now scopes person rows to current grants
and medication changes to current snapshot visibility, including visible
health-event references. Medication deletion metadata is captured before
dependent records are removed; tombstones use current grants. The final
isolated Rails suite passed 6,102/0, RuboCop passed 1,892 files with no
offenses, docs build passed, and the integrated Rails-backed Rust privacy
contract passed 1/1. The Rust API remains absent, so Rust server parity and
the memory target are not yet verified.
