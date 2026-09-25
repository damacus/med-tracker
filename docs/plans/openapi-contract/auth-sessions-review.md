# Account-session checkpoint review

The initial six compiled HTTP tests failed on absent routes in isolated project
`mtcontract-3ed44576cbe1435a`, recorded in
`1790373634_task_api_442df8.log`. A preceding compile failure is not behavioural
RED evidence.

Independent source review found and corrected three issues before the acceptance
run: API sessions without a household membership must not authenticate; mobile
OAuth account-session access must not depend on an active household membership;
and revoking another session must record the target session's household in its
audit event. The latter also requires the correct database tenant context for
the security-audit insert. Regression assertions cover these distinctions.

The implementation uses account and credential-type filters, locks selected
credentials before revocation, and commits revocation and audit writes together.
Listing batches membership lookup. App-token lifetime uses calendar months;
mobile login lifetime uses the configured inactivity and absolute-age limits.
Audit metadata excludes raw credentials and hashes the user agent. Unknown or
already revoked logout credentials return an empty success response.

The first consolidated HTTP run passed all seven test groups in project
`mtcontract-36173ca9734645ad`, recorded in
`1790374341_task_api_442df8.log`, with successful cleanup. The orchestrator
inspected the terminal log. Runner regression checks also passed, including
session-suite dispatch and failure cleanup. The final run passed eight groups
in project `mtcontract-580c08dfe7ae4d14`, recorded in
`1790374763_task_api_442df8.log`, after adding malformed identifiers,
expired-login targets and simultaneous revocation checks. The API unit suite
passed 15 tests, including the calendar-month cap boundary.

The tests exercise strict response fields, account and credential namespaces,
invalid authentication, durable revocation, repeat logout, target-household
audit writes and rate-limit rejection without revocation. This is evidence for
the three session operations only, which now have adequate documented-contract
proof. It does not prove HTTP refresh-token
rejection, UI behaviour, memory targets or completion of the wider API goal.
