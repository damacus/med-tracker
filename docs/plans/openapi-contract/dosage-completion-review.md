# Dosage completion review notes

Initial behavioural RED at `1467df53`: five existing groups passed and four new
groups failed in project `mtcontract-734187e1b2d54dc1`, log
`1790375418_task_api_d3a3cd.log`. Failures were administrator and permitted-member
read denial, parent single-dose value left intact on dosage creation, and an
invalid quantity accepted with 201. The orchestrator inspected the terminal log.

Review requirements for GREEN: medication visibility must filter both list rows
and total count; writes require owner/administrator; parent and option writes,
audit and sync events must remain atomic. Household then medication then dosage
is the shared lock order. Several tracked options must aggregate stock and
reorder values; untracked options must not contribute. Rejected duplicate
defaults and aggregate overflow must preserve all affected data and events.

Legacy nullable values remain a pending user decision and are not resolved by
these tests. Do not mark the five operations fully verified while that response
contract remains unsettled.

The first implementation run passed ten HTTP groups in project
`mtcontract-92344c9e319541b7`, recorded in `1790375869_task_api_d3a3cd.log`, with
successful cleanup. Review then requested direct regressions for the parent
Medication version and merged-record validation on legacy-row updates. These
follow-up tests failed as intended in project `mtcontract-971e9f9434634163`, log
`1790376059_task_api_d3a3cd.log`: missing parent version and a 500 response for
an incomplete legacy-row update. After correction, the final isolated run in
`mtcontract-1d35ec94c1de4bf9` passed all eleven groups, recorded in
`1790376248_task_api_d3a3cd.log`, with successful cleanup.

The parent version now shares the dosage write's request ID. Merged-record
validation rejects invalid partial repairs with 422 and rolls back the write;
a repair producing a valid record succeeds. Inventory totals use one database
aggregate query instead of loading every dosage into application memory.

Shared household app-token authentication is also incomplete; see
`household-auth-followup.md`. Passing dosage role tests with session credentials
must not be reported as proof of every supported credential type.
