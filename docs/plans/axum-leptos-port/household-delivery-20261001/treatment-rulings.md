# Treatment browser behaviour

The existing API remains the authority for treatment permissions and writes.
These rulings guide the browser forms without changing public API contracts.

## Pausing treatment

A pause form must not silently act on a treatment changed after the user opened
it. Keep the original treatment identity and edit token. If they no longer match,
reject the save and preserve the reason, note and submission key.

Canonical pause creation currently ignores public If-Match. The browser can use
a private typed request extension carrying the original source kind, portable ID
and ETag. Under the existing household transaction lock, reauthenticate, resolve
the visible source and check current person management permission. Honour an
authorised exact replay first. Then compare the original source identity and
the ETag of its canonical representation before a new mutation. Calls without
the private extension retain current public behaviour; JSON or headers cannot
forge it.

Missing or blank browser tokens return 428. Stale or mismatched originals return
409 with the submitted draft intact. Do not replace an old token with the result
of a fresh read. Keep lock order, audit, sync and idempotency behaviour intact.

Canonical resume already supports the pause period’s If-Match. Require and
forward that original period token in the browser; it is distinct from the
treatment source token. Carry the original pause period ID as well, including
when retrying a completed resume. Also apply the private browser source guard
under the resume transaction: a changed treatment must not be resumed from an
old form even if its pause period has not changed. Reauthorise and honour an
authorised exact replay before checking new-state preconditions. Public calls
without the private extension retain their existing behaviour. Verify permission
loss, exact replay after the first successful save, conflicting repeated payloads,
controlled concurrent changes and rejected writes without changes to treatment,
periods or history. Public compatibility must remain tested.

## Documented optional response fields

The existing assignment baseline rejects `can_record` and
`eligible_stock_medication_ids`, which are documented optional OpenAPI fields.
Add them to the test helper’s allowed keys and check their boolean and integer-ID
array types when present. Keep the original required fields and rejection of
unknown keys. Apply the same repair to a schedule helper only if its current
source and failing result show the same discrepancy. This is a test contract
repair; no response or API behaviour change is authorised.
