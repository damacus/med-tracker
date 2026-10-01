# Household delivery continuation

User authorised autonomous completion on 1 October 2026 while AFK. Continue
OpenSpec `add-rust-household-medication-workflows`, starting at 14/20 accepted
tasks and published commit `5a7c984a354fc7cd3ad07a21e2d3c0cf320fb17e`.

## Current progress

People, locations, medication forms and dosage management are published.
Dosage management has passed CI. Stock management has passed its checks and
independent review; publication is next. Assignments, schedules and the final
combined checks follow it.
Rails stays operational throughout this work.

## Stable path and ownership

One persistent Sol 6.1 writer owns product changes, regression tests and fixes.
One persistent Luna Medium verifier exclusively runs installs, compiled checks,
Docker, fixtures and browser tests. One independent Sol reviewer reads source
and evidence. The coordinator owns Git, acceptance rulings and these records.
No overlapping writers or repeated broad inventories. Existing completed agents
are retained for context; only these three seats are active.

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

Each journey is a stop-and-assess waypoint: requirements review, quality review,
matching-source verification, then publish accepted work and advance. A failed
assertion goes back to the same writer; two failed evidence-based fixes require
a diagnosis ruling, not another blind rerun. Never weaken requirements.

The two-hour retrospective automation `rust-delivery-two-hour-retrospective`
was created at 17:07:45 BST (verified native creation timestamp); the checkpoint
is 19:07:45 BST / 18:07:45 UTC. It applies local process improvements
immediately and continues work. It is not a deadline or feature-dispatch stop.
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
