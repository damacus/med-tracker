# Prepare a clinical transfer

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

## Exact supported mapping

Input must be a complete household `medtracker.portable.v2` document with all
eleven arrays, their exact retained-serializer fields and matching manifest counts.
Unknown/missing fields or collections, authentication data, duplicate JSON keys,
duplicate/malformed portable identities, unresolved clinical references and
nonempty attachment manifests fail before output publication. Fractional values
must be decimal strings; floating-point JSON numbers fail rather than being
rounded. This checks structure; the API still validates clinical values and
current permissions.

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

## Dry-run, apply and replay

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

## Evidence and limits

`prepared-history-v2` uses the retained Rails household serializer with all eleven
collections and different current/historical stock labels. Its manifest and
extracted-entry serialization use the actual pure HostedExport builders. Their
hashes join the retained inputs checked in API CI. The acceptance case prepares
that input and exercises declaration denial, dry-run, apply, replay and restored
field/side-effect assertions. The unchanged v2 negative case still proves its
historical-location rejection.

This in-memory synthetic harness does not run database-backed export selection,
model validations or an installed deployment. It does not extract/restore ZIPs or
attachments. Profile JSON/ZIP backup remains separate under
[#2513](https://github.com/damacus/med-tracker/issues/2513). Remaining history
variants, negative authority/conflict evidence and final care-sharing compatibility
remain in [#2702](https://github.com/damacus/med-tracker/issues/2702).

Reproduce serialization/manifests with
`RUBY=/path/to/compatible/ruby rtk task test:portable-rails-fixtures`.
Library tests cover validation/private no-clobber output; `prepared_rails_` covers
the supported API transfer journey.
