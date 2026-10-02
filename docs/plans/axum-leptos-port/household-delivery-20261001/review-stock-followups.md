# Stock review follow-ups

Two stock problems remain for the final integration work. A household with more
than 500 visible dosage options can lose access to medication pages. A medicine
with only untracked dosage options cannot record a browser removal from its
remaining parent stock. The reported 255-character audit limit is not supported
by the current PostgreSQL schema contract. These findings do not change the
accepted stock report or interrupt assignment delivery.

This is read-only triage of the three live PR 2357 threads, read on 1 October
2026. No runtime, production edit or external review reply was performed.

## Medication pages above the option collection limit

Confirmed source finding, P2, recommended labels **rust, bug**. Keep it in G under
existing [2353](https://github.com/damacus/med-tracker/issues/2353), extending its
affected-route list rather than creating an unrelated issue. Thread
PRRT_kwDOPAWvwc6oG-qA, comment
[4159726017](https://github.com/damacus/med-tracker/pull/2357#discussion_r4159726017).

WebApi::collection in rust/api/src/web_pages/api_client.rs:210–233 requests at
most five pages of 100 and returns 503 unless the authoritative total has been
reached; COLLECTION_LIMIT at line 15 is 500. inventory.rs:15–21 reads the entire
authorised option collection. The medication list calls it at line 84 and detail
rendering at line 314. stock.rs:46–51 reads the same collection before filtering
to its selected medication. Therefore 501 visible household options prevent
those pages from rendering, even if the selected medication has no options.
The existing fail-closed behaviour avoids treating a partial collection as proof
of scalar stock; preserving that safety is required in the remedy.

G should supply complete authorised reads using the actual public query contract,
without an invented medication filter or silent truncation. Require a real
501-option regression covering list, detail and stock hub, including a scalar
medicine and hidden/foreign options. No new runtime result is claimed here.

## Removing parent fallback stock

Confirmed source finding, P2, recommended labels **rust, bug**. Place a narrow
browser compatibility repair and its regression in G after F, rather than
changing the canonical stock API or reopening accepted E evidence. Thread
PRRT_kwDOPAWvwc6oG-tG, comment
[4159726309](https://github.com/damacus/med-tracker/pull/2357#discussion_r4159726309).

web_pages/stock.rs:240–250 treats any option presence as requiring a selected
option with string current_supply. web/src/stock.rs:240–262 likewise renders a
dosage selector whenever options exist, but includes only tracked quantities.
With all options null, its choices are empty and blank dosage_id is rejected.

Canonical stock_removals.rs:504–551 instead permits omitted dosage_id when no
option has non-null current_supply and parent current_supply is finite. It
rejects insufficient parent stock, subtracts quantity from the parent, and
records the parent version and sync change inside the existing locked mutation.
OpenAPI:7989–7992 describes dosage_id as optional and omitted for medication-level
stock. Thus an existing finite 20 ml parent with only null options can support a
2 ml removal through the API, while the browser adapter rejects the same intent.

The UI should select parent fallback only when no tracked option exists and
parent supply is finite. Zero is tracked; a zero-supply option must still prevent
parent fallback. Require truthful parent quantity/unit and retained rejected
drafts, manager permission/CSRF, successful subtraction, exact replay and
changed-payload conflict, mixed/tracked-option denial, and all-null/null-parent
denial. Keep the canonical API's locked recheck authoritative under concurrent
option creation. These are recommended checks, not executed evidence.

## Alleged 255-character adjustment reason limit

Reject the reported cause as unsupported; do not label or file a confirmed
255-character product bug. Thread PRRT_kwDOPAWvwc6oG-vC, comment
[4159726493](https://github.com/damacus/med-tracker/pull/2357#discussion_r4159726493).

The actual migration db/migrate/20251112140002_create_versions.rb:37 uses
t.string :event without a limit. Current db/schema.rb:1251 agrees. No subsequent
event length alteration was found in the relevant migrations. OpenAPI:6539–6540
declares reason as a string without maxLength; inventory.rs:127–143 validates
that type, and 214–221 preserves a nonblank reason in the event. persistence.rs:72
inserts that string without applying a 255-character limit.

[PostgreSQL 18 character type documentation](https://www.postgresql.org/docs/18/datatype-character.html),
queried through Context7, states that character varying without a length
specifier accepts strings without a declared character limit. Rails string is
not evidence of varchar(255) for this adapter. The comment's proposed
300-character failure is therefore not established. A generated schema.sql was
not present in the retained locations inspected; this review does not claim an
independent live database introspection or long-reason execution.

There is a distinct robustness question: db/schema.rb:1262 and
db/migrate/20251113221210_add_performance_indexes.rb:7 index versions.event.
Very large incompressible event strings may encounter PostgreSQL B-tree entry
limits despite an unrestricted column. If investigated in G, use an actual
representative request and exact database failure before proposing a contract or
storage change. Do not silently truncate audit reasons or invent a public 255
limit from this rejected finding.

## Verdict

Requirements triage: two confirmed G follow-ups, one unsupported reported cause.
Quality/security triage: preserve complete authorised reads, existing locked
stock validation, original removal replay identity and full audit information.
Runtime verification of these new follow-ups remains pending. Accepted D/E
reports are unchanged; F remains the active implementation journey.
