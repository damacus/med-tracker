# Dose occurrence review

Devin SWE-2 Max reviewed the occurrence routes, model operations, called helpers,
schema and API contract on 6 October 2026. This document records decisions against
the actual retained implementation. Both review passes and their response-parity
corrections are complete.

## Corrections implemented

- Recording a take over a missed decision must return 428 when `If-Match` is
  absent, and 409 when the supplied version is stale. The retained occurrence
  controller also stores a keyed 428 response. Preserve those distinctions.
- List validation must retain the `validation_failed` code and date-range field
  errors. The occurrence adapter preserves those details.
- Reading a list must not acquire an exclusive source-row lock. Keep exclusive
  locking for mutations and current-access replay checks.
- The person lookup used for transition audit must explicitly filter the
  household, as the existing dose audit does. An account-linked person can be
  visible under a separate row-security rule.
- Invalid-occurrence responses return 422 with `Occurrence is unavailable` and
  no field-error member. Valid JSON with the wrong envelope returns a keyed,
  replayable 400 with `Invalid request body`.
- Source lookup and current access precede malformed query, body and signing-key
  errors. Missing sources return 404; an existing source with no configured key
  still fails closed with 500.
- Already-resolved decisions return `already_resolved`; stale versions return
  `sync_conflict`. Both retain their distinct 409 messages.

## Retained behaviour

The suggested success-only idempotency ledger would change existing behaviour.
`rust/api/src/dose_occurrences/responses.rs` stores invalid-occurrence and
validation responses, including time-dependent 422 responses and missing-version
428 responses. A retry uses the saved response; a new attempt needs a new key.
Preserve this policy and test it rather than silently changing the contract.

Inactive-source projection also matches the retained implementation. Historical
pause intervals affect which occurrences are expected. Taking a paused dose and
recording a missed occurrence have different domain rules; the review does not
establish that this difference is a defect.

The duplicate source lookup during mutation preserves authorisation before saved
replay and again inside the model operation. No extra refactor is required for
this delivery. Existing decimal and identifier validation remains authoritative
at the dose operation boundary.

A present but invalid or short `AUTH_SESSION_SECRET` remains an explicit,
fail-closed override; it does not fall back to configured settings. The existing
`index` permission arm remains available and requires View access.

## Verification

The original three HTTP journeys and all 108 care API tests pass. They prove
missed recording, current-version reopening, immutable take history, exact stock,
concurrent duplicate decisions, withdrawn access and full rollback after a
forced audit failure. Strict lint and the signing-key configuration unit pass.

Six additional HTTP tests cover signed-key tampering and source
binding, UUID and keyed replay headers, missing-version responses, date-range
errors, current action permissions, future decisions, trusted timezone gap/fold
projection and a read while another transaction holds the source write lock.
The unchanged implementation produced three genuine failures: missing-version
409 instead of 428, discarded date-range errors and a blocked list read. The
other six cases passed. After the minimum corrections, all nine occurrence
cases pass, all 114 care API cases pass, and formatter, compilation and strict
lint pass. Test containers, volumes and networks are removed.

The preceding frozen candidate passed full local CI, including all 178 desktop
and mobile browser cases in 10.6 minutes with no skips or flaky results. That
result remains separate from the final focused correction checks. No browser
page changed in the correction. No identity draft is included in this care
delivery. Publication, hosted acceptance, merging and deployment remain separate.

The final response-parity tests produced three further genuine failures after
successful compilation: invalid-occurrence message mismatch, source lookup
after absent signing-key failure, and source lookup after malformed query/body.
The other nine owning cases passed on the unchanged source. After correction,
session 26518 passed all 12 occurrence cases and session 23823 passed all 117
care API cases. The final assertions also reached and passed cached malformed
400 replay headers and the precise resolved/stale 409 codes and messages.
Formatter, compilation, strict lint (session 9487) and diff checks pass; owned
HTTP fixture resources are absent.

The full baseline with 114 care API cases and 178 browser cases precedes this
response-only correction. The final 12/117 focused results are separate evidence;
they do not claim another complete browser run or hosted acceptance.
