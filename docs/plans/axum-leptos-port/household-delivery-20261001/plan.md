# Household delivery continuation

User authorised autonomous completion on 1 October 2026 while AFK. Continue
OpenSpec `add-rust-household-medication-workflows`, starting at 14/20 accepted
tasks and published commit `5a7c984a354fc7cd3ad07a21e2d3c0cf320fb17e`.

## Stable path and ownership

One persistent Sol 6.1 writer owns product changes, regression tests and fixes.
One persistent Luna Medium verifier exclusively runs installs, compiled checks,
Docker, fixtures and browser tests. One independent Sol reviewer reads source
and evidence. The coordinator owns Git, acceptance rulings and these records.
No overlapping writers or repeated broad inventories. Existing completed agents
are retained for context; only these three seats are active.

1. D: split dosage API responsibilities and complete dosage-option management.
   Require authorised create/edit, original preconditions, rejected drafts,
   tracked/untracked stock, defaults, all locales, desktop/mobile and review.
2. E: stock adjustment, order and receipt, with authorised persisted read-back,
   audit/replay/concurrency checks and truthful tracked-option behaviour.
3. F: split assignment/schedule/pause modules as needed; complete direct and all
   seven scheduled assignments, edits and pause/resume. Verify taper boundaries,
   effective dates, timezone, permissions and history.
4. G: rerun relevant baseline failures from #2347, record Rails/OpenAPI rulings,
   finish the remaining portable-import responsibility split from #2348,
   promote accepted tests into CI and complete combined acceptance/publication.

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

The [early process review](retro.md) now governs execution handoffs: one explicit
writer READY/FROZEN notice, helper pagination/selector/token checks before
runtime, one validated copy, focused retries after test-only repairs, and fewer
coordinator metadata updates. The actual two-hour retro remains at 19:07:45 BST;
native automation is `rust-delivery-scheduled-retrospective`. Continue D/E/F/G
to completion; the checkpoint does not stop work.

E follows [stock-ruling.md](stock-ruling.md): parent absolute adjustment is scalar
only; dosage-option stock uses its existing option edit API with original
preconditions and atomic aggregation. Do not invent a dosage selector on the
parent endpoint or a recorded reason on option edits. Fix the existing dropped
scalar adjustment reason under [#2350](https://github.com/damacus/med-tracker/issues/2350)
with actual RED/GREEN and private audit evidence before promising reason history.
