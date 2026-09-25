# Location policy completion review

## Requirements verdict

The user approved preserving existing location permissions, deletion retention
and IP rate limits. These rules are now documented in OpenAPI before their
acceptance tests. This supersedes the pending-policy questions in the earlier
location write review.

The consolidated isolated run passed 18 write tests, eight read tests and one
HTTP rate-limit test. The independently inspected evidence is
`1790370643_task_api_7d11fd.log`, project `mtcontract-2e1eae7a3e7e43cc`.
The log records successful fixture cleanup. The assertions cover active-member
reads, owner/administrator writes, person manage grants, revoked and expired
grants, missing/invalid credentials, suspended memberships, and the absence of
a platform-support bearer bypass.

Deletion tests cover seven retained-history sources, a successful cascade with
dosage references, rollback after a late foreign-key violation, and a concurrent
history insert. The race accepts only retained history with a rejected delete,
or a successful delete with a PostgreSQL foreign-key rejection of the insert.

## Independent code review

The original cascade deleted dosage records before the schedules and assignments
referencing them. Review required realistic source-dosage references in the test
graph; that test failed before the deletion order was corrected.

The inactive-user test exposed a household permission response before credential
validity was checked. The regression remains in the suite. Authentication now
validates the account, membership and user before selecting the requested
household, using the existing account lookup policy.

Deletion locks the location and its dependent medication/source records before
checking history. Savepoints restore all affected rows on retained-history or
foreign-key rejection. Ordinary data access uses SeaORM; fixture SQL remains
confined to disposable test data.

Rate limiting uses atomic per-process counters and epoch-aligned windows.
An expiry heap and fixed bucket capacity bound storage. Review caught a trusted
loopback proxy inheriting the local-client exemption when its forwarding header
was absent or malformed. Exemption now depends on the direct peer being an
untrusted loopback client; resolved proxy addresses receive normal counting.
Untrusted peers cannot select their counter through forwarding headers.

The real HTTP test verifies the strict 429 envelope and numeric rate headers.
Controlled-clock unit tests cover thresholds, expiry, concurrent increments,
operation-specific limits, proxy handling and bounded counter capacity.

Code-quality verdict: no outstanding blocking finding in this reviewed change.
The approved policy completion tranche has passing acceptance evidence. This
does not establish complete behaviour coverage for all eight location methods,
nor completion of the wider API port. The operation evidence map records any
remaining untested cases. Memory targets and UI/PWA parity are separate work.
