# Final household checks and refactors

Start after assignment and schedule acceptance and publication. Finish the known
browser bugs, make the remaining large files easier to maintain, and put the
accepted tests into CI. Keep Rails operational and preserve public API behaviour.

## Fix the remaining browser bugs

For [#2353](https://github.com/damacus/med-tracker/issues/2353), prove the failure
with 501 visible dosage options. Cover inventory, a medication with options, a
medication without options, dosage forms and stock screens. Read every required
page through the documented authorised API. Check pagination consistency; a
partial result cannot establish that a medication has no options. Do not invent
a medication filter absent from the API contract.

For [#2358](https://github.com/damacus/med-tracker/issues/2358), prove that a
browser stock loss fails with finite medication stock and only blank-stock
options. Offer the medication-stock choice in that case. Any tracked option,
including zero, requires a specific option. Check actual stock and history,
insufficient stock without changes, permissions, replay and translated
desktop/mobile forms. Reuse the existing loss API.

Check adjustment reasons at the audit boundary: the alleged 255-character column
limit is unsupported, so do not add an arbitrary API restriction. Confirm a
300-character reason saves, then check a larger varied string against the actual
indexed audit field. Only classify a larger-value failure as a defect after an
observed result. If confirmed, preserve the full reason in an existing suitable
audit field and retain the public reason contract.

## Repair the test commands and compatibility checks

The coordinator owns Task and CI changes for
[#2359](https://github.com/damacus/med-tracker/issues/2359). Show that command-line
selectors reach the runner and actually choose the requested HTTP target. Keep
default browser selection and separate fixed-clock dashboard checks intact.
The wrapper repair passes its focused tests and selected all fourteen requested
HTTP tests in the real assignment run. Publish that
repair with the assignment work; adding accepted journeys to CI remains here.

Complete the documented rulings for [#2347](https://github.com/damacus/med-tracker/issues/2347)
in [later-journeys.md](later-journeys.md) and [parity-rulings.md](parity-rulings.md).
Repair obsolete test helpers without weakening the contract. Record fixture
defaults, decimal formatting, timestamps, optional fields, cache directives and
permissions before expensive runs. Preserve microsecond ETag precision.

## Split the remaining mixed-purpose files

Follow [remaining-module-review.md](remaining-module-review.md) and
[issue #2348](https://github.com/damacus/med-tracker/issues/2348) for
portable imports, doses, dose occurrences, invitations and OAuth. Establish
the affected observable baselines before moving code. Preserve comments,
function bodies, exports, transaction ownership, lock order, reauthorisation,
replay, audit and sync. Preserve mail delivery ordering and authentication checks.
Run relevant checks afterwards and obtain independent review. Keep cohesive
responsibilities together rather than targeting an arbitrary file length.

Run these existing tests before and after each split:

| File being split | Required tests and behaviour |
| --- | --- |
| Portable imports | `openapi_portable_writes`, `portability`, encrypted export and readback |
| Dose recording | `doses`, `dose_write_api`, `dose_mode_transition_api`, affected `source_capabilities_api`, sync and replay |
| Dose occurrences | `openapi_dose_occurrences`, `doses`, `schedules`, affected sync and replay, fixed-clock dashboard |
| Invitations | `openapi_invitations`, `invitations`, configured mail-failure behaviour |
| OAuth | `oauth`, `auth`, `openapi_auth_sessions`, `medication_mobile_oauth_api` |

The complete compatibility runner does not select every test listed here. Check
the actual target names and results; a passing general run cannot replace a
missing baseline. Record existing failures before repairing them, then run the
same observable checks before and after moving code.

## Final acceptance

The verifier owns runtime checks and screenshots. The reviewer independently
checks fulfilled requirements and quality/security. Cover the combined household
journeys, fixed-clock dashboard, affected API compatibility suites, five-language
desktop/mobile evidence, Rust source gate and documentation. Run permission
changes in separate fixtures or last where safe.

Check that horizontal scrolling makes the last mobile navigation link usable;
successful treatment screenshots show clipped text, while the shared CSS permits
scrolling. Verify the actual gesture before classifying this as a navigation bug.

Improve the known missing-version error guidance: `If-Match is required` currently
uses the translated generic form error. Add focused rendering tests in all five
languages, then map it to the existing guidance for reopening an incomplete form.
Keep unknown errors safely translated and preserve public API messages.

Promote accepted tests into CI and wait for applicable published checks. The
coordinator owns commits, pull,
push, PRs, issue status and final handoff. Merge, deployment, authentication
cutover and scanner bypass remain outside this work.
