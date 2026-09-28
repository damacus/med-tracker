# Invitations and profile review

The fixed scope remains 118 operations. Independent review is complete for
invitations and profile/avatar operations. Runtime acceptance passed; publication
is the remaining checkpoint for recording these eleven operations as complete.

## Invitations

Independent review checked manager and credential restrictions, tenant-scoped
queries, fresh authority under mutation locks, token hashing and response
redaction, strict input, ordinary keyed replay, SMTP configuration and rollback,
acceptance retries, linked care relationships, permission versions and audit /
sync effects.

Review required one request ID across acceptance response, security audits,
PaperTrail versions and sync events. The implementation now threads that ID
through acceptance. The regression assertion passed in the final new invitation
target, alongside SMTP rollback and linked relationship assertions.

SMTP failure rolls back rotation and does not cache a successful response.
Ordinary successful keyed retries avoid another delivery. SMTP acceptance and
database commit are separate operations: a crash between them is not proven to
provide exactly-once delivery.

Final invitation acceptance passed 6/6 new scenarios in
`mtcontract-356be5db04ee44af` and 8/8 older scenarios in
`mtcontract-0031f0aadffc4146`. The latter includes the corrected Rails resend
error code. Scoped database assertions preserve membership and sync checks
without depending on unfinished read routes. Separate projects prevent fixture
and mail contamination: a second full provision in one database is invalid
because some fixtures have fixed unique GTINs. Invitation GET/POST/DELETE rate
probes passed in `mtcontract-573d934362884cfc`.

The final target also proves keyed creation and revocation do not duplicate
domain effects. Validation errors are cached consistently: retries receive a
fresh matching response/header request ID, and a changed payload conflicts.

## Profile and avatars

Initial review found that profile writes reused a person loaded before locking,
account preferences were not serialised across households, and date-of-birth
updates omitted timestamps and existing audit/sync effects. The production
owner corrected these and reused the shared JSON idempotency implementation.

Avatar review verified authorisation before byte access, safe disk key handling,
service matching, nonblocking disk operations, replacement failure nonmutation,
and cleanup that cannot delete a blob still referenced by another attachment.
The initial upload returned 500 because its audit insert omitted a required
timestamp; this is fixed. Final acceptance in `mtcontract-8e4627295ae3484d`
passed 6/6 new and 4/4 older tests, including upload/download/delete, current
authority, MIME and size validation, and failed replacement nonmutation.
The additional GET storage-failure assertion passed in
`mtcontract-8c4efbcfcbfb4649`. Failed physical file cleanup may leave orphaned
bytes; no guaranteed physical purge or crash-atomic filesystem/database write
claim is made.

All cited projects cleaned up successfully. The exact staged production
snapshot passed formatting, Clippy and 20 unit tests. Runner dispatch, failure
propagation and cleanup checks passed. The final whole-API suite and final CI
remain separate completion gates for the overall 118-operation goal.
