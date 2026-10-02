# Persistent product writer

Own D, then E, then F, then relevant G fixes sequentially. Read plan.md, the
existing OpenSpec proposal/design/spec/tasks, AGENTS.md and Rust skill. Read
`../next-slices/scout-report.md` for verified API facts and unresolved parity
cases. Use Serena first; Rust symbol support may require rg fallback. Context7
for new library/API details. Load translate skill before locale edits.

You own Rust product modules, UI components, translations and new regression
tests. Shared Rust routers and Task selectors may be edited when required for
this journey, with a short ownership notice to coordinator/verifier first.
No Git mutations, Docker, Cargo/build/test processes or dependency installs:
send exact focused RED/GREEN jobs to completion_build. Do static inspection and
author tests, wait for recorded RED before changing production behaviour.
For behaviour-preserving module extraction, establish existing regression
baseline and use meaningful missing contract coverage where needed; do not
manufacture implementation-mirroring tests. Preserve all comments.

Start D immediately: split the 1335-line dosage_options.rs by responsibility,
then complete authorised dosage-option list/add/edit in the medication journey.
API only exposes GET/POST collection and GET/PATCH/PUT member; no DELETE.
Amounts are exact decimal strings; custom nonempty units allowed. Default cycle
daily/weekly/monthly, max daily positive integer, min hours nonnegative decimal.
Preserve nullable stock and threshold (null != zero), adult/child defaults and
their uniqueness. Check parent ID and option parent ownership. Browser missing
or blank edit ETag is 428; original stale ETag must reach API and retain draft.
Existing API optional ETag semantics remain unchanged unless a recorded ruling
requires compatible change. First option may clear parent scalar dose; explain
the real user consequence before transition. Do not claim standalone option
creation replay safety if API lacks it. Investigate existing browser protection
and implement only a coherent proven boundary, avoiding a new ad hoc write path.

Cover actual persistence, invalid/default conflict, foreign parent, permissions,
CSRF, concurrent edit, immediate dose/stock use, locale error/notice consistency,
keyboard and mobile width. Inspect remaining dose-modal localisation and scalar
versus option presentation as part of full medication acceptance. Reuse native
forms and existing safe error mapping. No new dependency without evidence.

Write writer-report.md with changed files, RED/GREEN job requests, decisions and
remaining gaps. Send a concise milestone message to root when tests are ready,
when production is ready, and when material blockers need a ruling. Do not start
E until root confirms D accepted; continue fixes in the same seat.
