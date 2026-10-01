# dashboard report

Status: implemented after actual RED; independent static review passed; nine C
HTTP cases and bounded browser acceptance passed. Final combined independent
review and coordinator publication remain pending.

Baseline: eccf62aace3aab81bf871b380c964ff0ee28e7b6.

## Cause and approved behaviour

Before this change, the dashboard converted any profile read error to 503. The existing
active view-only fixture has a linked account, user, person and membership, plus
a view grant to another person; it has no own-person grant. The profile API
correctly returns 403 because its own-person read requires a current grant.
That profile permission does not remove the member's access to permitted
dashboard people, sources, stock and history.

Coordinator security decision: browser dashboard handling may treat profile
403/404 as absent optional preferences. Read `/me` before the optional profile;
retain its mandatory success requirement. Defaults use the existing configured
timezone, ordinary mobile shortcuts, visible-person-first selection and generic
greeting. No inaccessible profile details are disclosed.

Every authorised clinical read remains mandatory, including people, schedules,
assignments, medication takes, inventory and dose occurrences. Profile 5xx and
all other profile errors retain 503. No profile API, authorisation policy,
grants, fixture visibility or projection changes are proposed.

## Evidence and coordination

Read the project Rust references and team charter. Serena is active with Ruby
language support only, so Rust discovery used targeted source reads. The test
owner has the existing fixture identity and real browser login helper; no
own-person grant will be added to manufacture a pass.

No compiled/runtime checks, Docker commands or builds were run by this owner.
The build owner recorded the unchanged-product RED before implementation.
Independent source review passed the approved 403/404 contract.

C-RED-001, frozen digest `422566e6`, stopped at the test's hidden-person
reference lookup: line 39 expected 200 but received the correct 404 for the
primary credential without a grant to that hidden person. The dashboard
request at line 41 was never reached. The raw log is
`/tmp/medtracker-C-RED-001.log`; this is setup failure, not dashboard 404 or the
required dashboard 503 RED. The test owner has the existing feed credential
which can read that reference without adding grants. Production remained
unchanged until the corrected RED.

C-RED-002 reached the dashboard after corrected reference setup. It returned
503 with `Dashboard unavailable` where the test expected 200 at line 88; one
test executed and four were filtered out. The build owner recorded unchanged
pre/post input digest `c8787997...`, verified copied source
`7034ac78e59d9cdadf221fa6396c0ab179daf9f07b09c31f1979f3780bed3838`, fixture
SHA `b46e575cd41915f5c1fd91fe60f7e7fda2404ff786cfda25a60ed441b4116427`,
and project `mtcontract-39a668d2b4294294`. Full evidence is
`next-slices/evidence/C-RED-002-full.log`.

## Implementation

Only `rust/api/src/web_pages/dashboard.rs` changed. The mandatory authenticated
`/me` read now precedes profile. Profile errors matching HTTP 403 or 404 use
`Value::Null`, allowing the existing safe preference defaults to apply. Every
other profile failure still returns dashboard 503. Clinical reads, person
selection, task projection, stock, history, mobile rendering and API policy are
unchanged. No comments were added or removed.

`dashboard_projection.rs` needed no change. Independent requirements and
quality/security source review passed with no material finding. The reviewer
also audited the actual C-RED-002 evidence.

## Bounded acceptance

`ABC-GREEN-003` passed all nine C HTTP cases, within 28 passing combined HTTP
cases. C covers the active limited-view member without own-person access,
absent optional profile, profile server failure, required authorised people
read failure, administrator without person grants, managing carer, parent with
a public minor grant, owner with authorised self profile, and suspended/revoked
membership denial. The parent case passed after its test owner corrected the
reference setup; no dashboard policy or fixture grant shortcut was introduced.

The API run used the verified immutable source copy
`52eca56e5adfd378c891d042055fa840be8d21e90eba8c1379bbc970b3d25b32`
and disposable fixture SHA
`3682d6bd8af84185d7d7cbdfcc8ea708a8d086d472aea8c16a7c913c03768e50`.
Raw evidence: `next-slices/evidence/ABC-GREEN-003-full.log`.

`ABC-BROWSER-GREEN-004` passed all 35 tests across seven selected browser files
against the Rust API listener. Both C desktop and mobile cases passed, keeping
profile access forbidden while displaying only permitted clinical data. The
combined browser run also passed dose stock/history refresh at desktop/mobile,
CSRF/replay denial, taper recording, focus restoration and foreign-credential
isolation. It used the same verified 361-path freeze (digest prefix
`4fea2556`) and source copy above. Raw evidence:
`next-slices/evidence/ABC-BROWSER-GREEN-004-api-final-full.log`.

Fresh C screenshots are
`docs/screenshots/ABC-BROWSER-GREEN-004-52eca56/household-completion-dashboard-desktop.png`
and
`docs/screenshots/ABC-BROWSER-GREEN-004-52eca56/household-completion-dashboard-mobile.png`,
within 40 fresh combined screenshots. Screenshot capture supplements the
passing browser assertions; it does not expand the accepted scope.

Final combined independent review is in progress. The separate legacy dashboard
fixed-clock regression is queued; no result is claimed for it. Broader issue
`#2347` audits have not been rerun. This evidence accepts the bounded C behaviour
and combined selected regressions, not the whole household programme or legacy
dashboard suite. The coordinator owns final acceptance and publication.

Only this report changed during acceptance handoff. Dashboard source remains
stable; this owner ran no source, runtime, Docker, build or Git changes during
the handoff.
