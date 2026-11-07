# Prepare a clinical transfer

## Household ZIP import

The 0.6 Administration import journey is intended for a complete MedTracker
0.5 household ZIP. Local synthetic API and browser acceptance passed on
7 November 2026; a real hosted 0.5 export has not yet been tested. See the
[evidence and remaining checks](clinical-portable-evidence.md#household-zip-acceptance-status).

1. Sign in to 0.6 with a fresh account and create or join the destination
   household. Keep the existing 0.6 login, profile and person IDs: importing
   clinical data does not replace credentials or create authority from source
   carers.
2. In Administration, select import from MedTracker 0.5, upload the household
   ZIP and enter its passphrase. Choose and explicitly confirm the source IANA
   timezone, such as `Europe/London`; the export does not establish it.
   Inspect the inventory and map each source person to the intended destination.
3. A new dependent needs a fresh guardian declaration by an eligible guardian.
   For another adult, review any profile differences; where consent is
   required, the recipient responds through their own account. Current
   household, clinical and consent authority still applies. A source carer
   relationship cannot grant it.
4. Upload the **same ZIP and passphrase** again to create the reviewed preview.
   Check each person and shared record, relationships, blockers and omissions.
   Acknowledge each exact omission you accept; do not proceed if required
   decisions or dependencies remain unresolved.
5. Re-enter the preparation passphrase, select the explicit confirmation
   checkbox and confirm the ready preview. Read the resulting report: processing
   can return a completed or failed state. Compare expected and actual records
   per person and for shared data, and check that each expected relationship
   has a verified outcome. The existence of both records alone does not prove
   their relationship. Keep the report for review and use it to check replay.

If the source, choices, consent or destination state changes, a stale preview
must be replaced with a new preview and reviewed again. A failed apply rolls
back clinical writes. Restored historical administrations do not deduct current
stock again. If confirmation returns an uncertain response, read the report or
safely retry the same confirmation; do not assume that a response failure means
the import failed.

The API uses the common `/api/v1/households/{household_id}/data_imports`
prefix for inspection, preview, consent, confirmation and report operations.
See the [OpenAPI contract](openapi.v1.yaml) and
[client tools guide](client-tools.md). Personal keys need current care scopes;
the recipient reads and decides their own consent request.

The ZIP adapter accepts a bounded household export, not every possible 0.5
dataset. Compressed and total expanded archive size are limited to 64 MiB,
with at most 1,024 entries, a 16 MiB payload and a 1 MiB manifest. Attachments
are checked against their manifest and itemised as unsupported omissions;
obsolete person-location links and other unsupported fields are also itemised
where supported by the adapter. Unknown fields, unsafe archive entries, invalid
counts, unresolved required references and invalid values can block the import.
Required clinical dependencies cannot be waived by acknowledging an omission.

Decimal values must fit their destination precision without changing value.
For example, some stock fields have two decimal places, while a medication dose
stored as binary floating point must be exactly representable. Schedule
configuration numbers have the fixed-point and 28-decimal-place limits
described below. Review reported field outcomes and source dependencies; this
is not a promise of arbitrary precision or universal compatibility.

## Offline preparation

This is a separate, strict workflow. The ZIP adapter's itemised omissions do
not extend or relax `portable:prepare`; its extracted-file mapping and
rejections below remain unchanged.

The offline `portable:prepare` Task prepares extracted `portable.json` and
`manifest.json` from a `medtracker.household-export.v1` archive for the existing
Rust v3 import API. It reads no database and makes no network/API requests.
Use an existing private directory and protected passphrase file:

```fish
rtk task portable:prepare PAYLOAD=/private/export/portable.json MANIFEST=/private/export/manifest.json OUTPUT=/private/export/prepared.json < /private/export/passphrase.txt
```

It writes one complete package at a new path, with Unix permissions `0600`,
without overwriting a file or symlink. A private temporary file in the destination
directory is written and synced before publication through tempfile's
`persist_noclobber`. This does not promise power-loss durability or universal
atomic rename semantics: a crash can leave a second temporary hard link on some
platforms. See the [tempfile persistence contract](https://docs.rs/tempfile/latest/tempfile/struct.NamedTempFile.html#method.persist_noclobber).

The passphrase comes only from stdin. One final LF/CRLF is removed; other spaces
are preserved. Empty, multiline, invalid UTF-8 and over-4096-byte secrets fail.
Payloads are bounded to 16 MiB and manifests to 1 MiB, with the existing encryption
limit also enforced. Status and safe errors go to stderr; stdout is empty on
preparation. The example executable supports `--help`. Exit codes: 0 prepared,
1 validation/I/O failure, 2 usage error.

### Exact supported mapping

Input must be a complete household `medtracker.portable.v2` document with all
eleven arrays, their exact retained-serializer fields and matching manifest counts.
Unknown/missing fields or collections, authentication data, duplicate JSON keys,
duplicate/malformed portable identities, unresolved clinical references and
nonempty attachment manifests fail before output publication. Clinical fractional
fields must remain decimal strings. Within `records.schedules[].schedule_config`,
JSON numbers such as taper amounts `0.5` and `1.0` remain numbers. A numeric token
must use fixed-point notation, fit a 96-bit decimal coefficient and at most 28
decimal places, and retain exactly the same decimal value through ordinary JSON
parsing and serialisation. Exponent notation, numbers outside these limits and
numbers that would round fail with a precision/representation error. Equivalent
spelling can normalise (`1.00` to `1.0`); numbers are never converted to strings.
This bounded support does not promise arbitrary-precision JSONB transfer.
Duplicate keys (including escaped spellings), reserved serde JSON protocol keys
and nesting of 128 or more levels are rejected. This checks structure; the API
still validates clinical values and current permissions.

The mapping explicitly selects `medtracker.portable.v3` and removes only validated
**empty** `people.location_portable_ids`. Nonempty obsolete relationships remain
unsupported and require source review; they are never discarded or retained as
hidden metadata. All other clinical values, eleven collections, portable IDs,
source metadata, exact decimal strings, microsecond history, health-event links
and historical stock labels remain unchanged. Unavailable original pause actors
remain historical references and never become destination authority.

The `medtracker.clinical-preparation.v1` package contains `bundle`, using the
unchanged portable AES-GCM/PBKDF2 envelope, and a separate `conversion_receipt`.
The receipt records versions, hashes of exact input bytes/prepared plaintext,
counts, and the number of empty fields removed. These local hashes record the
conversion; they do not authenticate an export or prove source database
completeness. Counts remain private metadata; no clinical field values appear
outside the encrypted bundle.

### Dry-run, apply and replay

Review the receipt and [field matrix](clinical-portable-evidence.md). Pass **only
the package's `bundle`**, with the same passphrase, to the existing
[portable import API](portable-data.md). The wrapper and receipt are not API
input. `jq '.bundle' prepared.json` selects the encrypted envelope; construct the
existing request with that envelope and separately explicit initial guardian
declarations where required.

Review API dry-run errors/conflicts before apply. Preparation creates no account,
canonical Person identity, guardian authority, disclosure approval or
RestoreHistory/adoption provenance. Current self/guardian, household, RLS,
immutable-history and atomicity checks remain server authority. Historical restore
must not deduct stock, enqueue live notifications or replace clinical timestamps
with conversion time. Replay uses the existing import rules.

### Evidence and limits

`prepared-history-v2` uses the retained Rails household serializer with all eleven
collections and different current/historical stock labels. Its manifest and
extracted-entry serialization use the actual pure HostedExport builders. The
acceptance case prepares that input and exercises declaration denial, dry-run,
apply, replay and restored field/side-effect assertions. The unchanged v2
negative case still proves its historical-location rejection.

`prepared-taper-v2` uses the same retained serializer and archive builders with
numeric taper amounts. Its preparation/decryption and API dry-run/apply/replay
regressions preserve the numbers' types and values. Negative cases reject rounded
long fractions, excessive precision, exponent notation and fractional JSON numbers
outside schedule configuration.

This in-memory synthetic harness does not run database-backed export selection,
model validations or an installed deployment. It does not extract/restore ZIPs or
attachments. Profile JSON/ZIP backup remains separate under
[#2513](https://github.com/damacus/med-tracker/issues/2513). Remaining history
variants, negative authority/conflict evidence and final care-sharing compatibility
remain in [#2702](https://github.com/damacus/med-tracker/issues/2702).

The fixtures are frozen committed data; the generation harness was retired with
the Rails archive. Library tests cover validation/private no-clobber output;
`prepared_rails_` covers the supported API transfer journey.
