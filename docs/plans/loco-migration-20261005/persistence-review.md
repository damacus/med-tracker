# Guarded adoption review

Devin SWE-2 Max reviewed frozen P2 source and runtime evidence on 5 October 2026.
Requirements passed; technical quality was good; no demonstrated vulnerability
was found. The source-only review did not independently run PostgreSQL.

The frozen suite passed 12 persistence tests, nine runner tests and four database
helper checks. Rust lint and documentation checks passed. All owned fixtures were
cleaned. This is local evidence, not acceptance of the whole persistence section.

The owning writer has implemented three findings together with P3: make the 177-row
Rails ledger assertion explicit, prefer the system schema in search_path, and
test/refuse exported adoption on caller-owned transaction executors. The private
standard migrator still runs inside the guard's own transaction.

Final focused checks pass: persistence 16/16, tenant access 7/7, Take 7/7,
runner 9/9 and database helpers 4/4, with formatting and Clippy clean. The queue
capture adds exactly 19 named objects; actual restricted-role ServerAndWorker boot
passes. Devin completed the combined review and approved the stated bounded scope.
Its repair and coverage batch passes tenant access 11/11, care 20/20, formatting
and Clippy. Devin accepted the repaired production source. Its requested normal
member coverage passes: linked stock succeeds, an unlinked same-signature medicine
is excluded and explicitly selecting it adds no clinical effect. Coordinator review
checked that coverage-only addition and verified unchanged production hashes.
Full CI passed before that final test addition and is repeated on the final freeze.

The combined review findings are checked against the retained implementation:

- The canonical membership role is `administrator`, not the fixture's `admin`.
  The fixture correction and negative access cases belong to the current batch.
- Acting-user attribution intentionally follows the account's first linked person,
  as Rails `Account#person` and the retained Rust authentication code do. A
  membership's optional person need not belong to that account. Substituting it
  would change identity semantics; a regression test covers cross-household use.
- The captured `rule:public.household_audit_ledger_entries._RETURN` contains the
  baseline view's complete SELECT and household predicate. View definitions are
  covered by that rewrite-rule comparison; the baseline has this one view rule.
- Clinical `client_uuid` replay permits omitted optional fields, matching the
  retained Rust implementation. Explicit mismatches still conflict. Tests cover
  changed source defaults separately from strict HTTP Idempotency-Key handling.
- Explicit domain request-zone selection, duplicate conflict targeting and
  tracked-stock timestamps now pass their regression tests. HTTP binding remains
  required during transport integration. The existing
  process-zone fallback does not prove account-zone HTTP integration.

The combined source review is retained at
`/private/tmp/medtracker-p3-take-devin-review-20261005.txt`. Review the changed batch
and its focused evidence together before running full CI. The completed re-review
is retained at `/private/tmp/medtracker-p3-repair-devin-review-20261005.txt`.

The remaining notes do not change this capability's acceptance boundary. The
account lockout clock follows the retained authentication behaviour. The cycle
helper is called, though its forwarding layer can be simplified when timing is
next edited. Entity types match the captured database: medication dose_amount is
double precision and reorder_threshold is NOT NULL numeric(10,2). Remaining small
care branches belong to subsequent care coverage within this migration. Authenticated
HTTP identity, provenance, account-zone binding and strict Idempotency-Key handling
remain required transport work; no follow-up issues replace those requirements.

The catalog comparison's actual application scope is recorded in
[persistence-brief.md](persistence-brief.md). We retain every baseline identity,
definition, owner, ACL and policy comparison; we do not claim a general PostgreSQL
administration audit. The baseline defines no application casts, collations,
full-text configurations, foreign servers, replication publications, custom
languages, transforms, aggregates, statistics or materialized views.

Other findings were assessed against current evidence:

- The scoped offline Cargo update ran successfully. Its lock delta added only
  declared local dependency entries; no package version changed. The review's
  conjecture that this command necessarily fails was not reproduced.
- A failed rollback can obscure the original error, but remains a rejection.
  This is a diagnostic limitation, not an accepted database write.
- The status tripwire is tested against the pinned SeaORM version. Ledger capture
  duplication fails closed if it diverges; no new abstraction is required here.
- The supported test entrypoint is the owned runner, which scrubs ambient database
  and Compose settings. Direct manual invocation with forged ownership environment
  values is outside that entrypoint's safety contract.
- Advisory locking coordinates adoption callers; it does not prevent unrelated
  administrator DDL. Controlled administrative access is already required.

The full temporary review is retained at
`/private/tmp/medtracker-p2-devin-source-review-20261005.txt`. Review the bounded
fixes with completed P3 before final publication and acceptance.
