# Medication management delivery

Continue from accepted medication journey `6fc6dc24`. The full cutover goal
remains active. Deliver medication creation/editing, stock changes, removal
history and ordering through the shared API, followed by the existing web
wizard/edit/refill workflow.

## API requirements and ownership

Reuse the nine Rails-derived cases in `medication_stock.rs`; append that
target to the canonical 53-case suite. Preserve decimal JSON strings,
conditional writes, scoped locations and medication visibility, current
permissions, exact stock arithmetic, replay protection and request audits.
Add focused contracts for tracked dosage totals, concurrent stock/replay,
current-role checks and atomic conditional updates.

- Product owner: `oauth_product_sol`, new `medication_management.rs`,
  `stock_removals.rs`, `audit_logs.rs`; necessary `lib.rs`, `entities.rs` and
  `audit.rs` integration; own report.
- API test owner: `journey_auth_tests`, contract tests and canonical target
  selection; fixture changes only if necessary; own report.
- Browser test owner: `journey_browser_tests`, a shared Rails/Rust management
  browser test and dedicated task/runner selection; no canonical API edits.
- UI design owner: `journey_auth_product`, read-only wizard transaction/API
  dependency design until product paths and interface are agreed.
- Reviewer: `journey_auth_review`, independent requirements/security/code
  review and own report.
- Runner: `journey_auth_runner`, immutable acceptance execution and evidence.
- Orchestrator: interpretation, acceptance, ledger, integration and push.

Product implementation starts after demonstrated HTTP RED. Tests and product
work proceed concurrently in disjoint paths under the team charter. Use a
disposable acceptance snapshot when runtime builds would freeze other work.

## Intended correction: audit access

Rails `AuditLogPolicy#index?` and its scope require a household manager, and
the web controller enforces that policy. The API audit controller omits the
authorization call; its nominal managers-only request test covers only the
positive case. Rust must require the current owner/admin household role.
Ordinary membership or a delegated person-management grant does not authorize
the household audit list. Add positive, negative and foreign-household tests.
Do not preserve this Rails omission as a compatibility requirement.

## Browser delivery

Preserve the real wizard: medication details, person and as-needed assignment,
starting stock, review/save, subsequent edit and refill. Use the existing
disposable household/person fixtures and an independently created medication.
Run the same behavioural journey against Rails and Rust. Establish the API
dependency needed to preserve combined medication/assignment validation and
transaction semantics; do not replace the wizard with a simpler form merely
to fit currently implemented endpoints.

## Acceptance

Require the selected HTTP suites, focused security/concurrency contracts,
shared management browser baseline and Rust result, existing login and dose
browser regressions, applicable lint/unit/docs checks, visible desktop/mobile
verification and independent requirements/code-quality review. Record known
Rails defects separately. Commit and push accepted work; no production
deployment, data migration, merge or Rails retirement is included.
