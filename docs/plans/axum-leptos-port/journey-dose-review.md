# Direct dose and history: independent review

Review scope: household medication-take collection GET and direct POST, their
stock and audit effects, and the matching public HTTP contract. Scheduled
occurrence actions, general CRUD and the later browser session boundary are
outside this API review. Baseline: `2a8b4c1b`.

## Rails authority and intended correction

- `MedicationTakesController` scopes history to the current household and
  granted people. Direct POST resolves a household-visible schedule or person
  medication and requires the person's `record` grant.
- `MedicationAdministration::RecordDose` rejects a dose more than one hour in
  the future, invalid amount, unavailable source, blocked schedule/timing and
  unsuitable stock. The selected stock must match the source medicine and
  selected dose; more than one eligible stock location needs an explicit
  selection. `MedicationTake` persists a decimal dose and decrements stock
  through its creation callback. The source and stock are household bound.
- `medication_takes.client_uuid` is globally unique. Rails returns an existing
  take for a repeated UUID before validating the incoming payload. That lets a
  changed request receive success and is a known defect. The Rust contract
  must accept only an identical, currently authorised replay and reject a
  changed-payload replay without changing the take or stock.
- Rails accepts `dose_unit` in POST input but ignores it and always persists
  the source's effective unit. The orchestrator classed silent acceptance of
  a contradictory explicit unit as a defect: Rust must reject it with 422 on
  the first write or 409 for an existing UUID. An omitted unit defaults to the
  source; an explicitly matching unit is accepted.
- The new changed-UUID response uses HTTP 409 with generic `conflict`, matching
  Rails' generic conflict code. The existing `sync_conflict` code belongs to
  conditional sync/occurrence actions. Root OpenAPI currently describes Rails
  direct POST and has no 409 for this corrected defect; its eventual Rust
  contract reconciliation is part of cutover documentation, not a claim that
  Rails already returns 409 here.
- The existing direct-take contract exercises precision, replay, pagination,
  person privacy, role denial, foreign stock/source rejection and invalid
  time. Its stock helper uses floating-point assertions, so the new contract
  needs exact decimal HTTP and database assertions.

## Product design gates

The product writer proposed one database transaction with a household lock,
fresh source and `record` grant checks, selected inventory/dosage-option lock
before stock sufficiency, exact decimal validation, globally unique UUID
handling, take insert, stock decrement and domain audit. On replay, compare
the stored source, timestamp, amount, unit and selected stock with supplied
fields after checking current household and `record` access. An exact retry
returns the original take even if stock is since exhausted, the source paused
or current defaults changed. An explicit contradiction returns a generic
conflict. A UUID belonging to another household must disclose no take details.
The current request still gets its own
`api.request` audit event, including on an authorised replay or rejected POST.

Review will verify that `numeric(10,2)` overprecision and range are rejected
before persistence; that all selected stock reads occur under the required
row locks; that current person permission is checked on replay; and that a
failed or contending write leaves no partial take, stock or domain-audit
effect. Tenant settings and restricted role must apply inside the transaction
that performs those operations.

The orchestrator confirmed that direct schedule and person-medication sources,
tracked dosage-option stock and timing/overlap restrictions are in this POST
slice. The product writer will implement their real rules. Invalid timestamps
and unknown or foreign sources must retain their established error and privacy
behaviour.

During the first Rust source pass I returned the following issues to the
product writer for correction before freeze: hidden-person source existence
leak, partial-unique-index conflict handling, backdated cycle counts,
retired-source response, stocked-location selection, tapering effective dose,
stock audit effects, membership changes during the household-lock wait and
OAuth activity on rejected writes. The first three were corrected in the
subsequent draft; the rest need final-source review. These are source findings,
not acceptance evidence.

The later source pass confirmed corrections for those items, local cycle
boundaries, replay after exhausted stock or changed source state, full tracked
dosage binding, and generic database error logging. The initial MedicationTake
create version lacked clinical content; the final source now writes a snapshot
and `[null, new]` changes for the administered amount, time, source and stock,
with actor/request context in the same transaction. The frozen HTTP contract
checks this content and absence of bearer secrets. This is a source and test
review result; the runner has not yet proved the complete combination.

## First-party browser design review

The separate `journey-web-boundary.md` proposes an in-process shared API
router, signed DB-backed browser session, boundary-level CSRF and common
tenant binding. This avoids duplicating dose rules. Before implementation,
same-origin checks should compare against the configured public origin rather
than a client-controlled Host; the internal request must retain the relevant
Origin, method, cookie and CSRF headers and must not inject an already-trusted
authentication context. A rendered dose form needs one stable client UUID
across double submission and validation rerender. Cookie requests must
recheck the DB session, account, household membership and person grant. These
are design review conditions, not browser acceptance evidence.

## Runner isolation review

`api:browser-rails` provisions its own disposable Rails project and runs the
Playwright sidecar in the `web-test` service network namespace. The Rust API
runner remains a separate service namespace with no API host binding. The
Rails Compose base still publishes an unused ephemeral loopback port for
`web-test`; this is a nonblocking cleanup opportunity, not a fixed-port
collision. A plain `ports: []` overlay would not reliably remove a merged
Compose port entry. The runner's owner marker and generated project ID scope
cleanup to the current run. The fake-runner Rails failure case currently
checks project cleanup but does not assert `api:contract-image-remove`, unlike
its API browser failure case; adding that assertion would strengthen the
cleanup regression test. Neither observation changes the dose API verdict.

## Acceptance evidence and verdict

The initial public HTTP GET case failed with 404 against unchanged API at
baseline `2a8b4c1b`. The first combined run then passed five of six dose
cases. Cross-household reuse of a global `client_uuid` returned 500 instead
of generic 409. SeaORM's empty `ON CONFLICT DO NOTHING ... RETURNING` result
surfaced as `RecordNotFound`; the focused fix maps that case to conflict only
at this insert site. The request handler rolls back before writing the
failed-request audit in a fresh transaction.

The final canonical `task api:acceptance` run passed all six direct dose cases
and all 36 HTTP cases, plus five existing OAuth browser smoke cases. Its
92-file scoped Rust API, contract, browser and runner input manifest matched
before and after at SHA-256
`a770367c50f152d5d2f1ac81479c38872bcb6a57a401f9c7fff936e6e3a00eea`;
the disposable fixture was SHA-256
`e714aa7652c082643ca1398cb87b124b7f705a8f27daf0955718c34adcbecf33`.
The manifest did not cover every Rails fixture-builder source file; the
captured fixture content hash and Rails test image identity supplement that
bounded inventory.
The dose cases cover exact decimal take/stock/history, current role and person
access, foreign source and stock denial, changed-payload and cross-household
UUID conflict, exact replay after exhaustion, concurrent same-UUID convergence,
schedule and tracked dosage stock, timing denial, pagination, and take,
stock and request audit effects. The OAuth browser checks do not exercise a
Rust dose browser flow.

**Requirements verdict: passed for the bounded direct dose POST and history
GET API slice.** Corrected Rust behavior deliberately differs from Rails on
changed UUID payloads and contradictory supplied units. The separate Rails
browser baseline proved desktop/mobile 1.25 ml submission and displayed stock
reduction from 20 ml to 18.75 ml, but its history and strict Escape focus
checks exposed Rails UI/test gaps. A first-party Rust browser dose journey,
scheduled occurrence actions, general CRUD, and root OpenAPI reconciliation
remain outside this acceptance.

**Code-quality verdict: acceptable for this slice.** Source review found no
remaining blocking transaction, privacy, precision or audit issue after the
fixes above. Focused compile, format, lint and six unit checks passed in the
product lane; the frozen HTTP run gives independent behavioral proof. The
current suite does not directly force a role change during a lock wait,
different-payload concurrent UUID contenders, or a Europe/London DST boundary;
these remain useful targeted regression cases. The unused ephemeral Rails
host port and the missing fake-runner image-removal assertion are nonblocking
runner maintenance findings.
