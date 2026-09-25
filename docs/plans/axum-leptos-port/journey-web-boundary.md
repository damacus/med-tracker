# First-party web boundary for the medication journey

The direct-dose API and first-party session boundary are accepted. The next
bounded tranche completes shared source reads and the medication UI.

The shared-read product owner owns `rust/api/src/read_resources.rs`,
`read_entities.rs` and the generic read-audit helper in `audit.rs`. The web
product owner owns route integration in `lib.rs`, cookie household discovery
in `oauth.rs`, the new `web_pages.rs` API transport, and `rust/web/src/`.
The API test owner owns `web_session_api.rs`; the browser test owner owns the
medication journey suite and its dedicated Task/runner wiring. Each retains
its fixes and report. Review and execution remain independent. No database
queries belong in the web crate or medication web handlers.

Shared-read implementation follows its recorded four-case Rust HTTP failure.
Cookie household discovery and medication UI changes require their own
recorded failures before implementation. The dedicated commands are
`task api:web-session-acceptance` and `task api:browser-rust`.

## One API for web and native clients

Leptos remains server-rendered for this journey. Web handlers call the same
Axum API routes used by Android and iOS; they must not query domain tables or
repeat dose rules. Use a separate API router as an in-process Tower service,
passing real HTTP method, path, headers and JSON body. Render the resulting
API representation after its transaction finishes. Do not call the combined
web router recursively or make a loopback network request to the same server.

This keeps one application process and one bounded database pool. It also
preserves the HTTP authorization, validation, idempotency, audit and response
boundary. Axum's documented Router/ServiceExt oneshot composition supports
this approach. Bound collected response bodies and list sizes.

The current medication representation intentionally has no embedded person
or source records. The web phase therefore also needs the existing shared
API's visible people, schedules and person-medication read endpoints, with
their current permissions and pagination. Port those reads as prerequisites
to rendering the assignment picker; do not insert database access into web
handlers or invent a browser-only dose source endpoint. The authoritative
schemas are the root OpenAPI document and Rails serializers.

## Browser credentials

Extend the existing signed browser session backed by
`account_active_session_keys`; do not place API bearer or refresh tokens in
HTML, JavaScript, URLs or browser storage. Direct `/login` creates a signed,
short-lived CSRF-protected web login intent. An OAuth request retains its
validated registered-client intent and resumes consent. Client-supplied
return URLs cannot choose arbitrary redirect destinations.

The existing OAuth login POST may omit Origin and Referer: its signed intent
and exact CSRF token remain mandatory. Reject any supplied foreign origin
on login. This exception does not apply to authenticated cookie API writes
or logout, which require trusted origin evidence as well as session CSRF.

Allow cookie authentication at the shared API boundary while retaining
bearer authentication for native clients. An explicit Authorization header
must be validated as supplied; an invalid bearer must not silently fall back
to a cookie. Resolve current account state, session lifetime, factor policy,
membership, household lifecycle and person permissions on every request.
Refactor common membership binding rather than introducing a third copy of
authorization logic.

Preserve the account's primary actor identity across households: Rails
`Account#person` explicitly orders by person ID, and its API derives the user
from that person. Household membership determines access independently.
Do not switch to another active household-linked user to bypass suspension
of the primary user. Multiple linked people permitted by the schema do not
by themselves establish a defect in this deliberate account-level identity.

All unsafe cookie-authenticated API requests require a session-bound CSRF
token and trusted same-origin checks. Apply this at the API routing boundary
so a future write handler cannot accidentally omit it. The server-rendered
form validates its CSRF token, then forwards the signed session cookie and
CSRF header to the same API. The internal transport grants no extra privilege.
Bearer-only native requests retain their current contract.

Use the configured public origin for same-origin checks, never an arbitrary
incoming Host. Preserve the necessary Origin/Referer/Host semantics when
forwarding internal requests; do not inject a pre-trusted AuthContext.

## First usable path

Successful standalone login leads to the current household dashboard,
household selection when needed, or the supported empty-household state.
Preserve the established primary-fixture
redirect to `/households/{slug}/dashboard`; do not force a new picker into
that journey. Authenticated pages expose a session-bound CSRF token in
`meta[name="csrf-token"]`, distinct from the short-lived login-intent token.
Selection is a route choice, not a
permanent grant cached in a cookie. The medication page renders the shared
API response; recording a dose submits the shared dose API and redirects to
updated stock and history. Preserve entered values and display useful errors
when a write is rejected. Logout invalidates the current browser session and
clears its cookie, with CSRF protection; confirm Rails token/logout semantics
before deciding whether an independently issued mobile grant is affected.
Generate the dose client UUID with the form and keep it unchanged through
double submission and validation rerender. Generating a new UUID on every
POST would defeat duplicate protection.

The current shared source representations expose `can_manage`, which is not
permission to record doses. Rails source policies check a separate `record`
grant. The first owner journey may submit through the shared API and show
its denial, but matching action visibility for view-only users remains an
owned parity gap. Resolve it through a tested shared API capability before
claiming permission-aware UI parity; do not duplicate grant rules in Leptos.

## Required proof before acceptance

- A real browser signs in from `/login`, selects its household, reads its
  medication, records a decimal dose and sees the updated stock and history.
- Both cookie and bearer clients use the same dose behavior and permissions.
- Missing/wrong CSRF, cross-origin writes, expired/revoked sessions, foreign
  household/person access and changed membership are denied without writes.
- Repeated form submission is duplicate-safe; logout rejects a copied old
  browser cookie. OAuth login/consent still passes its existing suite.
- Desktop/mobile layout, keyboard operation, visible validation/focus and
  screenshots are inspected. Presence-only assertions are insufficient.

Use existing Rails browser assertions and current screenshots as the UI
authority. The unstyled early Leptos placeholder is not the target. Broader
MFA, account recovery, PWA/offline and remaining UI workflows stay on the full
goal; do not claim them from this path.
