# medication report

Status: 428 draft retention and stale-race correction implemented after their
actual HTTP RED results. Source formatted and reviewed; focused HTTP GREEN is
verified in ABC-GREEN-002 and ABC-GREEN-003. Browser GREEN is verified in
ABC-BROWSER-GREEN-004. Bounded slice B requirements PASS; final combined
independent review and coordinator publication remain pending.

Independent full slice B source review: PASS for requirements and code quality,
with no material security finding. The reviewer confirmed authorised second-read
errors, original final If-Match and original-mode draft retention. HTTP/browser
GREEN is recorded below.

Baseline: eccf62aace3aab81bf871b380c964ff0ee28e7b6.

## Slice B discovery

`save` reads the medication and ETag, then reads dosage options. Its scalar
override check uses the first ETag. If the first option commits between those
reads, the options response changes the form mode while the original ETag still
matches the stale submitted token. The handler returns 400 before the final
mutation can reject the stale token with 409.

The proposed correction re-reads the authorised medication ETag after option
discovery, before deciding whether submitted scalar fields are invalid. The
second ETag is only a stale-check input. The final API PATCH keeps the original
submitted If-Match. Conflict rendering keeps the submitted scalar/identity form
mode and all draft values.

The coordinator also approved retaining the submitted draft/token on 428. Only
the blank-token check trims whitespace; a usable token is never normalised or
replaced. The existing translated
`stock_removals.errors.invalid_submission` copy tells the user to reload the
form, without adding catalogue keys or raw error text.

## Deterministic test boundary

Approved by the coordinator for the test owner: a disposable fixture-only audit
trigger blocks the browser medication show audit on a unique advisory lock.
Medication show calculates its response body and ETag before that audit insert;
the browser cannot proceed to dosage options until its transaction commits.
The test holds the advisory lock, starts the authenticated browser POST, waits
for the expected ungranted lock, creates the first option through the bearer HTTP
API, verifies persisted parent/options/stock, then releases the browser request.
Use the exact browser session reference, actor and household for the trigger;
the audit record does not contain a medication ID. No sleep or production seam
is proposed. Trigger and lock cleanup must run on every failure.

Before-read coverage can use the existing stale scalar form sequence: capture
the browser form, insert the first option through the API, then submit the form.
The coordinator accepts this sequence for the first-read boundary.

## Recorded RED and bounded implementation

B-RED-001 reached all four contract cases. The pre-read stale form and current
malicious scalar override controls passed. The between-read case timed out while
creating the first option: the fixture's AFTER INSERT audit trigger had acquired
a household foreign-key lock which conflicted with the option writer's household
lock. The test owner changed that gate to BEFORE INSERT, preserving the boundary
after medication body/ETag calculation and before option discovery. That case is
was not a production RED.

The missing/blank token case established an unchanged persisted medication and
actual status 428, then failed `assert_retained` because the response contained no
form inputs. Test owner and independent reviewer confirmed this as the approved
draft-retention RED in `evidence/B-RED-001-http.log` (actual tee
`1790864926_task_api_browser-rust.log`). The coordinator released only that fix.

`medications.rs` now treats whitespace-only tokens as blank without changing the
draft token, and renders the submitted form mode/values at 428 using the existing
locale-derived reload copy. The original If-Match path remains unchanged.

B-RED-002's corrected BEFORE INSERT gate established the intended boundary and
observed actual 400 where 409 was required (contract assertion line 280). The
coordinator recorded source digest beginning `79c04b`, 299 validated files with
298 authored files matching the premanifest, and a pre-execution fixture hash
beginning `47f32fc`. This released the agreed race correction.

The handler now performs a second authorised medication GET after dosage-option
discovery. Its ETag is used for the precondition comparison only. The submitted
draft ETag still supplies the final If-Match. Both precheck and final API 409
responses render the original submitted scalar/identity mode and all draft
values. A concurrent change after the second read remains protected by the
existing locked API mutation. A narrow Rustfmt run formatted only the owned
handler file; no compiled/runtime check was run by this owner.

## Boundaries and next action

Only `rust/api/src/web_pages/medications.rs` and this report are owned. Serena
initial instructions and applicable Rust references were read; its active Ruby
server cannot extract Rust symbols. PostgreSQL 18 advisory-lock documentation was
checked through Context7. No build, test, runtime, dependency, Docker or Git
operation was run by this owner. No policy, catalogue, locking or idempotency
change is planned.

ABC-GREEN-002 exercised the Rust listener from immutable source digest
`a73a37679986879e9db631828957e1b1018ba48242add4b73a9df653e335c618`.
Luna validated all 301 copied paths against the frozen premanifest before tests.
The medication owner inspected `evidence/ABC-GREEN-002-full.log:135-141`: all four
slice B cases passed, proving both first-option insertion boundaries, retained
missing/empty/whitespace preconditions and current scalar override denial.
The medication lifecycle pair and navigation pair also passed at lines 144-155.
ABC-GREEN-002 later failed a delegated-token fixture interaction in the existing
`household_web` suite; the entire combined gate is therefore not accepted by this
focused HTTP result.

The independent full source review passed before this runtime evidence.

ABC-GREEN-003 repeated all four B HTTP cases successfully within 28 passing HTTP
tests, from source digest
`52eca56e5adfd378c891d042055fa840be8d21e90eba8c1379bbc970b3d25b32` and fixture
SHA-256 `3682d6bd8af84185d7d7cbdfcc8ea708a8d086d472aea8c16a7c913c03768e50`.
Luna reported this in `evidence/ABC-GREEN-003.log`. The job reached browser setup
but treated comma-separated filenames as one nonexistent filename, so no browser
assertion ran. This is an invocation failure, not a medication product finding.

ABC-BROWSER-GREEN-004 then passed all 35 browser checks against the same immutable
Rust copy digest `52eca56e5adfd378c891d042055fa840be8d21e90eba8c1379bbc970b3d25b32`.
The medication owner inspected
`evidence/ABC-BROWSER-GREEN-004-api-final-full.log:47-48`: the stale scalar draft
browser journey passed at desktop and mobile widths. Existing medication create/
edit decimal and warning journeys also passed at both widths (lines 59 and 64).
Fresh medication evidence is saved under
`docs/screenshots/ABC-BROWSER-GREEN-004-52eca56/`, including
`household-completion-medication-stale-desktop.png` and
`household-completion-medication-stale-mobile.png`. The runner captured 40 fresh
screenshots for the combined A/B/C browser run. This establishes bounded slice B
HTTP and browser GREEN; it does not complete dosage-option management in D or
OpenSpec task 4.3.

Source is stable and frozen. Final combined independent review, documentation
verification and Git publication remain coordinator-owned. No further medication
production edit is required by the current evidence.

## Subsequent medication work

The scout found that some existing untracked dosage-option metadata edits do not
update the parent medication version. First-option insertion does update it,
which supports this slice's stale-form scenario. The broader version semantics
need an explicit decision in slice D; this slice does not change them.

### D preparation only

The next bounded journey should list/add/edit dosage options under the selected
medication, using private adapter/renderer modules. The coordinator wires routes.
Every option read/write must verify that its `medication_id` matches the selected
parent after the existing authorised API boundary; household membership alone is
insufficient. Writes retain owner/administrator permission, trusted origin, CSRF,
household/parent/option locking and submitted option ETags. Missing/blank browser
ETags return 428 even though the current option API accepts omission.

Forms retain exact decimal draft strings, custom nonempty units and frequency,
adult/child defaults, dose cycle and limits. Nullable stock is untracked; zero is
tracked empty. Tracked options refresh parent supply/threshold totals in the
existing transaction. Acceptance needs default conflicts, positive and excessive
precision validation, multiple options, foreign parent/option mismatch and
immediate dose/stock read-back. No option removal route exists, so add/edit and
management must not imply deletion support.

Contract decisions before D production dispatch:

- Decide whether every untracked option metadata edit must update the parent
  version. Current updates can change the option alone; the slice brief promises
  parent version updates.
- Decide the public create/update replay guard. Standalone option writes have no
  idempotency lookup/store; default uniqueness does not prevent duplicate
  ordinary options. A browser submission UUID alone cannot establish safe replay.
  Any API mutation change needs coordinator ownership approval and deterministic
  same-payload replay, changed-payload rejection and single-effect persistence.
- Define scalar-to-option conversion explicitly. First-option creation clears
  scalar dose and may replace parent stock with tracked option totals. Do not
  silently copy or discard scalar stock in an editor.
- Confirm whether create needs an original parent-version precondition as well
  as option update's original option ETag, and what concurrent parent changes
  should reject. The current create API has no such explicit guard.

These are design findings from the scout and narrow source inspection. No D
feature code, test execution or acceptance is claimed.
