# Audit Evidence

## What the system records

MedTracker records two audit sources:

- `versions` contains PaperTrail create, update, and destroy evidence, including `object_changes`.
- `security_audit_events` contains authentication, authorization, support access, import/export, API, MCP, and other security events.

Both sources use the same versioned envelope. It identifies the event, outcome, time, household, affected entity, actor account/user/membership, active role, permissions version, authentication method, opaque session reference, request and trace identifiers, IP address, Pundit policy/query, support session, source row, redacted metadata, and retention decision. Secrets, bearer tokens, cookies, passwords, OIDC tokens, and raw session identifiers are not permitted in the envelope.

The web and API controllers establish and clear the request context explicitly. Background work is identified as system activity; it is not represented as user-authorized activity.

## Integrity boundary

PostgreSQL `SECURITY DEFINER` triggers append both source tables to `audit_ledger_entries` in the same transaction. Each household has an independent chain; events without a household use the global chain. Every entry binds the chain epoch, sequence, previous hash, canonical envelope, source payload, hash/schema versions, and retention decision into a SHA-256 hash.

The runtime application role can insert source events but cannot update or delete source or ledger rows. Household administrators read a tenant-filtered view. The audit exporter and verifier use separate database roles and credentials:

- `med_tracker_audit_exporter` reads ledger/checkpoint data, signs checkpoints through a one-way database function, and updates delivery receipts. It cannot read source or clinical tables or modify ledger history.
- `med_tracker_audit_verifier` reads source and ledger evidence and may insert the audit event describing an export. It cannot read clinical tables or alter existing source, ledger, checkpoint, or delivery rows.

Database-owner access remains a break-glass capability. PostgreSQL cannot independently audit a malicious database owner who controls the database and its logs. Every owner-level audit-table operation therefore requires an incident or approved change record in a separate system.

## Existing history

Rows that existed when the ledger was installed are chained in a distinct `legacy-baseline` epoch. A signed baseline manifest proves the bytes exported at that point and detects later changes. It does not prove that pre-migration history was complete or unmodified before the baseline. Documentation and exports must retain that label.

## Object Lock evidence

The exporter writes one deterministic, content-addressed JSON object per ledger entry and signed checkpoint to an S3-compatible Object Lock bucket. Uploads use conditional creation, SHA-256 checksums, server-side encryption, expected-owner checks, versioning, and retention metadata. Matching conditional-write conflicts are accepted only after the existing object, checksum, version, mode, and retention are verified.

Temporary storage failures leave the transactional outbox pending for retry. Configuration, checksum, duplicate-version, and retention failures stop automatic retry and require operator action. The web process has no signing key or WORM credential.

Governance mode is the default. `COMPLIANCE` mode requires `AUDIT_WORM_COMPLIANCE_APPROVED=true` after records-governance approval because its retention cannot be shortened. Object Lock configuration and permission validation is repeated while the exporter runs.

## Verification

Run:

```fish
task audit:verify
```

Inputs are supplied as environment variables:

| Variable | Values |
|---|---|
| `SCOPE` | `database`, `worm`, or `combined` |
| `FORMAT` | `human` or `json` |
| `HOUSEHOLD_ID` | Optional numeric household filter |
| `FROM` / `TO` | Optional ISO 8601 time bounds |

Exit status `0` means all selected evidence is valid, `1` means an integrity failure was found, and `2` means verification could not run because of configuration or runtime failure.

Database verification checks source-row equality, canonical payload parsing, sequence continuity, every previous-hash link, recomputed entry hashes, live chain heads, checkpoint targets, Ed25519 signatures, and retained public keys. WORM verification checks delivery completeness, object key/checksum/version, retention mode/date, missing objects, and duplicate versions.

Time-bounded verification validates selected entries and their predecessor links but does not compare a filtered range with the current chain head. Scheduled full verification is still required to detect tail truncation.

## Export

Run:

```fish
task audit:export
```

Set `OUTPUT`, optional `HOUSEHOLD_ID`, `FROM`, `TO`, and `FHIR=true` as required. The export contains deterministic native NDJSON and a signed manifest. `FHIR=true` also writes a FHIR R4 `Bundle` of `AuditEvent` resources. The native envelope remains authoritative for integrity; FHIR is an interoperability representation. FHIR defines AuditEvent as a security log and advises servers not to support update/delete because that compromises audit integrity: <https://hl7.org/fhir/R4/auditevent.html>.

Creating an export is itself written to `security_audit_events` without recording output paths or clinical content.

## Retention

Retention policy `clinical-security-v1` applies a ten-year default floor to clinical/security audit evidence. A related record schedule, legal hold, inquiry, litigation requirement, or approved local policy may require longer retention. This is a floor, not a universal claim that every record must be destroyed after ten years or kept forever.

Reaching `retain_until` makes evidence eligible for records-governance review. MedTracker does not automatically destroy expired audit evidence. A future governed disposal process must verify eligibility and legal holds and create an immutable destruction manifest.

The policy must be approved by the deploying organisation's records manager and DPO before production retention is locked. NHS records guidance applies different schedules to different records, requires appraisal at the end of the minimum period, and warns against unjustified continued retention: <https://transform.england.nhs.uk/media/documents/NHSX_Records_Management_CoP_V7.pdf>.

## Limits

These controls provide stronger technical evidence; they do not by themselves create UK GDPR, DSPT, DTAC, DCB0129, DCB0160, or other regulatory compliance. Governance, operating procedures, supplier controls, clinical safety work, and deployment-specific approval remain required.
