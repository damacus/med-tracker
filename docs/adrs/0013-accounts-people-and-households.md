# ADR 0013: Accounts, people and households

- Status
  Accepted (architecture; implementation acceptance pending)
- Date
  2026-10-15

## Context

Fresh Rust development supersedes the legacy account/person/user chain and
the older per-person household grant assumptions in ADRs 0002 and 0009.
Existing code and older design sections may still implement those assumptions.

## Decision

An account holds authentication, profile information and preferences and has
exactly one person. A person optionally has an account. There is no separate
user or profile table. People may belong to multiple households and may exist
without accounts. Minors cannot log in. Preserve `adult:0`, `minor:1`,
`dependent_adult:2`; minors and dependent adults have `has_capacity:false`.

A person creates a household or accepts a manager's invitation. Household
membership permits viewing and recording care for the household. Managers also
administer membership. Remove the intra-household per-person view/record/manage
gate. Sharing an existing person's care with another household requires approval
by the adult person or an authorised guardian. Management does not imply guardianship.

Active household belongs to the session/device and must be a current membership.
Forms and writes retain their original household binding after a switch. Recheck
revocation at every request and mutation boundary, including queued work.

## Alternatives

Separate profile/user records duplicate account identity. Person grants for each
household action retain obsolete complexity. Global active-household state causes
one device to redirect another device's writes. These alternatives are rejected.

## Consequences

Implementation must prove account/person cardinality, no minor login, guardian
approval, concurrent session isolation, stale-form household binding and revoked
membership rejection. Existing database constraints and tests require deliberate
updates; this record does not claim those updates have landed. Preserve live RLS,
audit attribution and restricted runtime roles throughout delivery.
