# Household delivery continuation

User authorised autonomous completion on 1 October 2026 while AFK. Continue
OpenSpec `add-rust-household-medication-workflows`, starting at 14/20 accepted
tasks and published commit `5a7c984a354fc7cd3ad07a21e2d3c0cf320fb17e`.

## Current progress

Paused at the user's request on 2 October 2026 until the rate limit resets.
Implementation and test agents have been stopped. The final branch is unfinished;
its application changes remain local and have not been published or merged.

People, locations, medication forms and dosage management are published.
Dosage and stock management have passed CI. Stock management is published in
[PR #2357](https://github.com/damacus/med-tracker/pull/2357) after its checks and
independent review. Assignments and schedules are published in
[PR #2362](https://github.com/damacus/med-tracker/pull/2362). Local checks and
independent review passed; GitHub CI passed. The final stock fixes,
compatibility checks, remaining refactors and runtime CI work are underway on
`codex/rust-household-final-20261002`.
Rails stays operational throughout this work.

The assignment checks passed 37 HTTP tests and 12 browser journeys, covering
editing all seven schedule types, pause/resume, rejected conflicts and actual
dose recording. Revoked-access checks and seven existing medication browser
tests also passed. Date tests found two taper problems: the dashboard displayed
the base dose, and the form's time fields did not set dose times. Both fixes now
pass their focused checks, and the final combined run passed 37 HTTP tests and
12 browser journeys again. Requirements review and the full Rust gate passed.
Final documentation checks passed and the change is published. The original journey list is
20/20; the final compatibility and refactor work below remains unfinished.

The final branch fixes two stock problems: large households can now open pages
with more than 500 dosage options, and stock losses work when options leave stock
blank but the medication still has a recorded balance. The combined HTTP and
desktop/mobile browser checks passed. These fixes remain unpublished and tracked
in [#2353](https://github.com/damacus/med-tracker/issues/2353) and
[#2358](https://github.com/damacus/med-tracker/issues/2358).

Minor users can now read authorised person pages and assignment history while
schedules remain restricted. Checks passed across all five languages on desktop
and mobile. The large import file has also been split and independently reviewed;
all five import tests passed before and after the move. Dose recording and its
tests have also been split and accepted. The occurrence split has passed six API
suites and thirty-five dashboard tests. The named schedule API suite exposed
pagination and numeric validation bugs in
[#2367](https://github.com/damacus/med-tracker/issues/2367) and
[#2368](https://github.com/damacus/med-tracker/issues/2368), alongside outdated
test data and assertions. The repairs pass all eleven active tests and six route
checks before and after the occurrence split. Independent review accepted it;
the final current schedule run has runner-emitted source and fixture hashes,
with that evidence limit recorded. The invitation refactor has also passed its
before-and-after checks and independent review. Sign-in API checks and all seven
login browser tests pass before the last refactor. That refactor is installed,
but its matching checks are incomplete. The existing deterministic stock test passed all
four cases, including rejection of a first-option race with its draft retained
and stock, versions and sync records unchanged. Reviewed extra assertions now
make the zero-to-one option transition explicit; their rerun remains.

The long stock-adjustment reason bug is fixed on the final Rust branch, with
six regression tests and the existing short-reason stock test passing. It is
tracked in
[#2365](https://github.com/damacus/med-tracker/issues/2365). The fix keeps the full
reason in the audit record and uses a short event label when necessary. Publication
is pending. The matching Ruby code still needs its own verification.

## Resume from here

Four final refactors are accepted: portable imports, dose recording, dose
occurrences and invitations. OAuth is installed but has not passed compilation
or its after-refactor tests. The compiler first found a private client ID; its
reviewed visibility fix is installed. The next check found three private handler
input types: `LoginForm`, `TokenForm` and `RevokeInput`. Their visibility-only fix
has passed independent review but is still a private proposal, not installed.
Both failed checks are retained in the verification evidence.

1. Install the reviewed three-type OAuth fix from
   `/private/tmp/household-g-20261002/oauth-input-visibility-proposal/proposal.diff`
   after checking that its source still matches. Run compilation, formatting,
   Clippy and unit tests before starting runtime checks.
2. Run the same five authentication API suites and seven standalone login browser
   tests used before the OAuth refactor. Run the four stock concurrency cases
   with the new zero-to-one option assertions. Those assertions are installed;
   their rerun is still outstanding.
3. Run the final fifteen-case household API selection, combined household browser
   journeys, fixed-clock dashboard checks and the final Rust/documentation checks.
   Verify the actual mobile navigation gesture and obtain independent review.
4. Commit and push the accepted application changes, open the follow-up PR against
   [#2362](https://github.com/damacus/med-tracker/pull/2362), wait for CI, and update
   the linked issues and review threads. No final application PR exists yet.

The household CI configuration now includes fourteen separate journeys; all
79 local policy tests pass. This configuration has not yet run in remote CI.
Keep the known legacy OAuth command setup problem in
[#2369](https://github.com/damacus/med-tracker/issues/2369) separate from the
supported test command. Preserve user-owned skills and existing screenshots.
The two-hour retrospective is complete; no replacement timer is needed.

## Stable path and ownership

One persistent Sol 6.1 writer owns product changes, regression tests and fixes.
One persistent Luna Medium verifier exclusively runs installs, compiled checks,
Docker, fixtures and browser tests. One independent Sol reviewer reads source
and evidence. The coordinator owns Git, acceptance rulings and these records.
No overlapping writers or repeated broad inventories. Existing agents are
retained for context; the three working seats are paused until the user resumes.

Progress and project reports must be clear to someone passing by. Lead with the
user problem, what changed and what still needs checking. Explain stock conflicts
and form behaviour in ordinary language. Keep commands, hashes, database details
and test mechanics in the evidence queue rather than the progress headline.

1. D — Dosage management: add and edit dosage options, show their stock correctly,
   and keep form entries when a save fails. Split the large API file into smaller
   modules. Check permissions, translations, desktop and mobile use. Accepted.
2. E — Stock management: adjust quantities, record losses, mark orders and receipts.
   Check that the saved changes are correct and conflicting saves cannot overwrite
   stock. Show each dosage option with its own quantity and unit.
3. F — Assignments and schedules: assign medication to a person, edit all seven
   schedule types, and pause or resume treatment. Check dose changes, dates,
   timezones, permissions and history. Split the large files before extending them.
4. G — Final checks: resolve the recorded compatibility failures, split the large
   import, dose, occurrence, invitation and OAuth files, and include the new tests
   in CI. Review the combined result and publish the remaining work. The
   [remaining module review](remaining-module-review.md) records the concrete
   boundaries; these are behaviour-preserving refactors, not new features.

After each journey, check that it meets the requirements, review the code, test
the final changes and publish the accepted work. Investigate failed checks before
retrying. If two attempted fixes fail, reassess the cause before changing more code.

The requested two-hour retrospective ran at 19:07:57 BST. Its improvements now
guide the work: check test data and commands before expensive runs, keep one
person responsible for running them, and repeat only the checks affected by a fix.
The completed timer was deleted. Resume the remaining work after the rate reset.
No routine AFK questions, global skill/memory edits, merge, deployment, Rails
retirement or security-scanner bypass are authorised.

## Verification discipline

Read actual Task selectors once before running. Browser filenames are separated
by spaces. Browser inventory route is `/households/{slug}/medications`.
Run grant-mutating HTTP tests last or in a separate fixture/token lifetime;
permission version changes invalidate old bearer tokens legitimately. Keep
fixed-clock dashboard tests separate from real-time household fixtures.
Use one fully awaited batched manifest per job, exact task command and input
digest, retain detailed assertion logs, and capture known screenshot baselines
before overwriting. No duplicate builds by readers. Network/runtime checks use
appropriate approved permissions from the outset. Keep Rails running and use
isolated disposable fixtures only. Do not print credentials or patient data.

## Product rulings

Reuse existing API and browser session/CSRF boundaries; no second write policy.
Receipt marks the order received; stock quantity is adjusted separately because
the API does not add stock on receipt. Do not invent partial receipt or DELETE
assignment endpoints. Option stock null means untracked; zero means empty.
Maintain exact decimal drafts and the original supplied If-Match. Missing
browser edit tokens are rejected without adopting a fresh token. API changes
need explicit compatibility evidence and authoritative OpenAPI alignment.

Global authentication parity and the parent PR security-scanner disposition
remain cutover gates, separate from completed household journeys. Label Rust
defects `rust,bug`, Ruby defects `ruby,bug`, and shared defects with both.

The coordinator accepts the three source-derived recommendations in
[parity-rulings.md](parity-rulings.md) for G: oversized integer People page size
clamps to 100; eligible-stock choice order follows location name then medication
ID; paused sources may project matching stock but actual recording must reject
without writes. Preserve malformed-query and unrelated-resource policies.
These are implementation rulings, not runtime acceptance; the verifier must
exercise their existing and additional no-write assertions on final source.

The [two-hour retrospective and earlier review](retro.md) govern handoffs: one explicit
writer READY/FROZEN notice, helper pagination/selector/token checks before
runtime, one validated copy, focused retries after test-only repairs, and fewer
coordinator metadata updates. The actual two-hour retro began at 19:07:57 BST,
verified with the live clock. Its fixture-precheck and capture-release improvements
apply immediately; the completed native timer was deleted. Continue D/E/F/G
to completion; the checkpoint does not stop work.

E follows [stock-ruling.md](stock-ruling.md): parent absolute adjustment is scalar
only; dosage-option stock uses its existing option edit API with original
preconditions and atomic aggregation. Do not invent a dosage selector on the
parent endpoint or a recorded reason on option edits. Fix the existing dropped
scalar adjustment reason under [#2350](https://github.com/damacus/med-tracker/issues/2350)
with actual RED/GREEN and private audit evidence before promising reason history.
