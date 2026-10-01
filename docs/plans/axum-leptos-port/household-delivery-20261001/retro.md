# Early process review — actual two-hour retro pending

Scope: this autonomous continuation and the immediately preceding A/B/C run,
using current conversation/checkpoint, ledger, verifier receipts and independent
review. Original checkpoint remains 18:07:45 UTC / 19:07:45 BST. Root misread
a verifier receipt labelled 18:09:47Z; the live clock was actually 17:12:02 UTC.
This review happened early, not at two hours. Root corrected the user update,
restored the native timer for the original checkpoint and will update this file
then. Verify timestamps with the live clock, not an agent's timezone label.
The user is AFK; do not pause for conversation.

## What is actually complete

The published A/B/C continuation has fully green CI. D source now implements
dosage option forms, safe five-language medication errors, translated dose
dialogs and a responsibility split of the old large dosage API file. Independent
static extraction and source/security reviews pass. Existing dosage API tests
passed 14/14 before and after extraction; fast compilation and Rust web tests
pass. Final D HTTP tests passed 4/4. Browser acceptance passed 26/34, with eight
immediate-use cases still failing. D is not accepted. Overall remains 14/20.
Stock, assignments and final parity are not yet implemented in this run.

## What worked

One writer retained product/test ownership and fixed the compiler error directly.
One verifier caught it before Docker acceptance. Independent review found missing
permission/immediate-use evidence and resolved three old parity disagreements
plus stock aggregation/audit semantics. The latest runtime copy was verified
before cleanup, and overwritten screenshot baselines were restored correctly.
Rust defects are labelled; the lost stock-adjustment reason is #2350, rust+bug.

## What cost time

Coordination was still too heavy: too many status/ledger updates and broad freeze
handshakes, with the coordinator spending time anticipating future details
instead of advancing the next concrete job. The verifier once used sandbox DNS
for a known network build, then retried with the already-required permissions.
A source-copy comparison raced cleanup; a redundant preparatory snapshot build
added work. Agent quiescence after a capacity error was mistaken for a source
freeze, causing a baseline to start before the final test notice. Two temporary
Sol capacity errors needed same-seat resumption.

The final browser helper fetched only the default first page of dosage options.
After English cases populated the fixture, later locales could not find their
new option despite successful saves. Independent source review supports this
harness diagnosis; it is not yet runtime proof of the fix. This repeats the
earlier wrong-route/selector/token-lifetime pattern: helper assumptions needed
validation before expensive acceptance. Current post-run input differs in only
that test file; preserve the immutable run's original provenance explicitly.

## Changes applied immediately

1. Reduce coordinator work to a job handoff, a diagnosis ruling and acceptance.
   Update the ledger at those boundaries; stop speculative future analysis and
   repeated report edits. Keep concise meaningful user updates.
2. Validate helper collection pagination, route paths, selector paths and token
   lifetimes statically before runtime. Fix the paginated lookup without relaxing
   any persistence, stock, replay, permission or locale assertions.
3. Use one explicit READY/FROZEN notice from the writer. Agent completion or
   capacity failure never releases ownership or certifies stable source.
4. Verify one captured input before cleanup. Do not build another snapshot solely
   for metadata. Each result certifies its actual copy; disclose post-run edits.
5. Run only the focused failed test file after a test-only helper repair. Reuse
   unchanged product regression evidence honestly, then execute the outstanding
   isolated permission case. Repeat broader checks only for changed product or
   new failures, and run the full relevant gate before publication.
6. Root checks the live clock at job boundaries and before time-based actions.
   The real two-hour retro remains a checkpoint, not a stop deadline.

Keep the existing writer, verifier and independent reviewer. No new team seats,
global skill/memory edits, merge, deployment or scanner bypass. Skills were
useful for ownership and review; their administrative burden must stay smaller
than the implementation. Global skill changes can be discussed when Dan returns.
