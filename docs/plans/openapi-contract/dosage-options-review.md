# Dosage-option checkpoint review

The five dosage-option routes were absent at `863585e0`. Four initial acceptance
tests compiled and failed on missing routes, recorded in
`1790371499_task_api_d3a3cd.log`. The first owner-path run passed five expanded
groups in `1790371959_task_api_d3a3cd.log`. Review then found that the provisional
guard also admitted administrators. A sixth test failed 200 versus 403 in
`1790372391_task_api_d3a3cd.log`. The owner-only correction passed six groups
in project `mtcontract-75054982d65b4cf2`, recorded in
`1790372564_task_api_d3a3cd.log` with successful cleanup.

The tests now cover strict response fields, numeric and portable identifiers,
decimal values, pagination and timestamp filtering, request validation, missing
and invalid credentials, foreign resources, conditional version checks and
concurrent updates. Missing If-Match is accepted because the specification marks
the header optional for these operations. The tests separately vary invalid
fields rather than relying only on requests with several independent errors.

Review identified possible silent decimal rounding. Parsing now requires exact
representation, and fixed-scale database fields reject values that cannot be
stored exactly. Rejected updates preserve the existing amount. Ordinary queries
and writes use SeaORM; list serialization fetches related medications in one
batch. Updates lock the dosage before comparing a supplied ETag. Constraint
failures roll back the savepoint before returning validation errors.

Code-quality verdict: no outstanding blocking finding in the reviewed owner
path. Requirements verdict: partial. The temporary administrator denial is a
guard against an unreviewed privilege expansion, not final domain policy.
Owner-only access is provisional because
the specification does not define dosage roles or visibility. Existing nullable
database columns also permit records outside the required response schema; the
implementation does not fabricate medical defaults for them. Authorization,
default-option switching and legacy-record policy must be resolved before these
operations can be marked complete. Per-operation omissions remain in the
coverage evidence map. This checkpoint reduces absent routes by five; it does
not claim five fully completed operations or completion of the 89-operation goal.
