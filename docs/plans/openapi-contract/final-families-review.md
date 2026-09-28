# Review of the final API families

Published checkpoint before these families: 102/118 operations, with 73/89 of
the original baseline complete. Findings below are acceptance gates, not
completion evidence. Runtime projects and final resolutions belong in the
per-operation evidence ledger after verification.

## Verified family runs

- Health events: `mtcontract-3988dc0ca26940d7`, 17/17 HTTP tests, including
  approved reassignment, sync visibility and replay after grant revocation.
- Exports and sync reads: `mtcontract-a53a91fb751a4b04`, 16/16 HTTP tests,
  including legacy feed and report-export cases.
- External integrations: `mtcontract-f89ffe6a4eae41c1`, 13/13 HTTP tests using
  deterministic provider doubles. APNs uses a verified PKCS#8 fixture key.
- Portable imports: `mtcontract-ff35a5ceb624423c`, 3/3 legacy HTTP tests,
  including delegated reference rejection and stale-session invalidation.
- Error envelopes: `mtcontract-3077605308354395`, 6/6 HTTP tests, including
  profile field errors and rate limiting through the non-loopback test address.

Each project completed cleanup. The final API regression evidence below
supplements these family runs; publication and remote CI remain separate checks.

## Regression selection

The complete API runner selects 39 isolated groups covering the fixed 118
operations. Legacy app-token tests run with the new app-token group; the legacy
administration group skips those four duplicate tests. Seven legacy lookup
checks retain authentication, view-grant, feature-gate and rate-limit proof.
The new external integration suite replaces old provider-specific and permissive
lookup expectations with deterministic provider protocol tests. UI, web-only
sessions, FHIR, SMART, MCP and platform administration are outside this fixed
OpenAPI scope.

The first full matrix exposed six failing groups. Review distinguished actual
defects from outdated test assumptions:

- Oversized avatar multipart bodies must return 422 and preserve existing bytes.
- Imports invalidate API sessions when access grants change; post-import readback
  uses the still-authorised mobile credential and explicitly checks session 401.
- Sessions bound to a non-operational household must return 401, matching app
  tokens. The account fixture now has two active household memberships, and
  SMART/FHIR credentials remain invalid for ordinary household endpoints.
- Settings GET has no documented ETag; only its incidental Rails-header
  assertion was removed, retaining response and authority checks.
- Person/location name errors and duplicate grant errors must identify their
  fields so clients can display actionable validation messages.
- Lookup rate checks must use the non-loopback Compose address; direct loopback
  intentionally bypasses the production limiter.

Each correction receives a fresh isolated acceptance run. The original failed
matrix remains recorded rather than being described as an uninterrupted pass.

The final frozen-production matrix completed all 39 groups. Thirty-seven passed
directly. Two test-only corrections required isolated replacement runs:

- Provider fake listeners moved from ports in Linux's usual ephemeral source
  range to ports 20085–20092 and 20097. The first and only serial bind of the
  Open Products Facts listener had returned `AddrInUse`; an ephemeral collision
  is the inferred cause, not a captured socket diagnosis.
- The legacy medication-read assertion for a session bound to a held household
  now expects the corrected 401 credential rejection. Its unrelated-household
  403 assertion remains intact.

No production source changed during this final matrix or these two repairs.

Both replacement runs passed with cleanup: external integration 13/13 in
`mtcontract-063183d4c96c40de`, and medications 33/33 in
`mtcontract-eaaf43a0c41348b2`. Both captured source digest
`07255ca1691cf4bf6d973bf125ff8dd84442af7c662fcb904fe4ad3317893680`.
The final result is 39/39 groups, 404 tests passed and zero remaining failures.
`coverage/final-acceptance.tsv` retains every final result;
`coverage/final-acceptance-initial.tsv` and `coverage/first-regression.tsv`
retain the earlier failures. Source digests include test and runner files, so
test-only repairs change that digest without changing the application code.

Final formatting, compilation, Clippy, 25 API unit tests, runner lint and
dispatch/cleanup tests, documentation build and Ruby fixture lint passed.
Live-provider delivery and UI/performance verification are not implied.

GitGuardian subsequently flagged the embedded APNs fixture key during PR CI.
The runner now generates a disposable PKCS#8 key per run, requires it in both
API configurations, never prints it, and removes it from the runner environment
after cleanup. Runner tests cover generation failure, valid key propagation and
output redaction. External acceptance passed again, 13/13 with cleanup, in
`mtcontract-ce76264854114a46` against source digest
`6e07c277a73c6ad6b7a4dde6b6cef8054c69c85ff146fb6eea3d6d10a6ea6cef`.
Only test wiring changed; the final manifest uses this latest external result.

## Health events

Reviewed strict field parsing, grant checks, household locking with fresh
authentication, stale writes, omitted-value preservation, version/change
recording, and medication-link reconciliation. Reconciliation must preserve
unchanged links and their historical medication-name snapshots. A regression
test captures the link identity, name and creation timestamp before a medication
rename and a subsequent event update.

The user approved changing an event's person when the caller can manage both
the original and destination people. Hidden or foreign destinations must remain
unavailable; visible destinations without manage access must be forbidden.
These cases now pass. Change feeds remove the event for clients losing access
without removing it for clients retaining access through its new person. Cached
batch replay rechecks both people, including every transition recorded for the
event in the original batch before a later deletion.

## Portable exports and sync reads

Review requires:

- Snapshot ETags must be usable by subsequent writes, including sync batches.
- Live medication change visibility must use current authorization and current
  associations; tombstones use their retained visibility metadata.
- Dose-occurrence change records must retain their documented complete
  projection, including source, window, outcome, linked take and identity.
- Pause actor references prefer the current membership's person, with imported
  references as a fallback.
- Export and mobile-snapshot business audit events retain record counts, mode
  and encryption status, alongside request audit.
- PBKDF2 and encryption run outside asynchronous request workers. Envelope
  validation, authentication before parsing, and size limits must be tested.

The synthetic Rust-export-to-Rails-import test is blocked by automatic approval
review pending explicit user approval. It targets only the disposable Compose
Rails service with generated fixture credentials and a fixed test passphrase.
No cross-runtime interoperability claim is justified until that test passes.

## Portable imports

Review identified these required checks in the initial draft:

- Resolve the person behind every take, pause and outcome source, including
  existing targets. A managed person elsewhere in the bundle must not authorize
  updates to another person's records.
- Share complete semantic validation between dry-run and apply. SQL type
  coercion and database constraints do not replace model rules for required
  fields, numeric ranges, dates, enums, capacity and relationship consistency.
- Validate minor/dependent-adult capacity independently of whether email is
  present. Required dose facts cannot be supplied through invented defaults.
- Preserve version events and person-scoped sync metadata for imported source
  children. Compare immutable history using normalized database types.
- Return the correct forbidden error code and the documented structured size
  rejection rather than treating an oversized body as malformed JSON.

Follow-up after the source-owner fix: delegated dosage import must validate the
existing dosage's medication as well as its incoming medication reference.
Otherwise an unrelated existing dosage portable ID could be reparented to an
allowed medication. Incoming assignment claims must not grant permission to
alter unrelated existing stock. The implementation now checks both dosage
parents and derives existing stock authority from existing managed-person
links. Compilation, unit tests and the encrypted Rails-fixture HTTP import
regressions pass, including the negative authority cases. The Rails import policy
allows delegated writes to linked medication; this intentionally differs from
the direct medication update endpoint's owner/administrator policy.

## External integrations

Review requires:

- Medication lookup retains catalogue metadata, related household-visible
  stock and review-prompt enrichment; empty placeholders are insufficient.
- Configured barcode source priority, cached/live Open Products Facts,
  curated records and Open Food Facts supplement fallback remain within the
  lookup operation's scope.
- Barcode matching preserves the single leading-zero 13/14-digit alternative.
- Provider JSON reads and trusted-source reads are bounded; redirects are
  disabled and trusted source URLs require HTTPS on port 443.
- FCM permanent-error detection scans typed details. APNs retains configured
  host/sandbox defaults when a device does not specify an environment.
- Sender-level push failures return the documented 503; individual transient
  device failures remain separate and do not delete registrations.
- AI output preserves sourced warnings and normalizes accepted numeric dose
  values to the response schema. Evidence must reference text actually fetched.
- AI suggestion audit stores a hashed identity and result counts/status,
  avoiding raw clinical content.

Provider acceptance uses deterministic service doubles exercising the real
outbound protocols. These results do not establish live-provider delivery.

## Sync batch writes

The Rails operation catalogue contains 34 action pairs across 11 resource
types. All 34 now have successful observable proof in the 13-test batch suite,
including public readback, rollback and replay checks.

Initial HTTP RED: `mtcontract-14d3bcb7b4834c66` returned 404 for an invalid
batch where 422 was required, and 404 for a missing import bundle where 400
was required. Both tests compiled and the isolated project was cleaned up.

Domain helpers must accept the existing transaction and retain their HTTP
handler's validation, authorization and side effects. Operations run inside a
savepoint while the outer transaction holds the household lock. A failed batch
rolls back all operation effects before its validation response and request
audit are stored; releasing and reacquiring the lock between these steps would
create an idempotency race.

Saved error replay retains the semantic error body while giving the response
and audit a fresh, matching request ID, consistent with the published location
and invitation handlers. Replayed success requires current authorization for
the original operations and resulting records.
