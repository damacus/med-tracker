# Medication browser journey: independent review

Scope: first-party medication list, detail, dose form and dashboard UI backed
by in-process calls through the shared `/api/v1` router. This report is
separate from the shared-read API review and from full Rails visual parity.

## Static boundary

The page transport uses the shared API router through an in-process bounded
`oneshot` request. It forwards browser Cookie, Authorization, Origin and
Referer headers, and the form's session CSRF token for the unsafe dose POST.
The API router retains its cookie origin/CSRF middleware, current household
authorization and direct dose transaction rules. HTML strings go through the
Leptos renderer and the self-hosted CSS/JS satisfy the page CSP. A successful
API read forwards the renewed session cookie to the page response.

Six material issues were identified in the first frozen source:

1. `record_dose` renders the next form with the submitted `client_uuid` even
   after success. A separate intended dose from that form reuses the previous
   idempotency key and becomes a replay or changed-payload conflict. Rotate
   the form UUID after success; retain it for a retry of an unsuccessful write.
2. The dashboard labels its history "Previous Doses Today" but selects a UTC
   date and compares it with the UTC `taken_at` string. The form and Rails app
   use the configured local time zone, so doses around local midnight are
   omitted or shown on the wrong day.
3. The HTML POST URL's medication ID is not checked against the submitted
   source before calling the shared API. A crafted POST to medication A can
   record an authorized dose for medication B, then render medication A with
   a success notice. Bind the selected source to the URL medication before
   mutation; retain API authorization and domain validation as the final
   boundary.
4. The initial stock picker treated matching medication names alone as
   eligible supply. Rails requires exact name, dose amount and unit, as well
   as current scoped and available stock. That heuristic could offer invalid
   same-name stock; the shared API would reject the write. The product owner
   has since narrowed the picker to the selected medication while an
   API-owned eligible-stock capability is built. Broader alternate-stock UI
   parity remains pending; a differently named medication is not eligible
   under the current Rails resolver.
5. The planned first path calls for a redirect to updated stock/history and
   preserving entered values with useful errors. Current POST returns a
   newly rendered detail page with no history, while 422/403 rerenders reset
   source, time and stock to defaults and shows a generic notice. The
   existing browser test only asserts 200 and one replay, so it does not
   settle the intended new-dose or rejected-form behaviour.
6. `WebApi::authenticated` accepts and touches cookie session A before
   forwarding an explicit Authorization header to the internal API, which
   can execute as bearer B. On HTML form POST, bearer authentication bypasses
   the cookie API CSRF guard, creating mixed-principal page and audit
   attribution. First-party HTML routes should reject an Authorization
   header before reading the browser session; `/api/v1` bearer access remains
   available separately.

The product owner corrected the first three boundaries: success and exact
replay now use a 303 redirect to an API-verified take notice and a fresh form
UUID; the dashboard uses the configured time zone; and the HTML POST checks
the submitted source's medication against the URL medication before dispatch.
The owner also moved stock choices to the additive API eligibility fields,
preserved rejected form values with an appropriate response status, and rejects
explicit Authorization on medication HTML before session lookup. A browser run
of the settled combined source subsequently passed seven selected medication
cases without skips.

The new source capability implementation received a bounded static review.
It derives `can_record` from the current household membership's unexpired
record/manage grant, keeps `can_manage` separate, and scopes stock candidates
to the current household and medication visibility. It shares the direct POST
signature, tracked dosage selection and stock sufficiency helpers, then orders
eligible IDs by location name and medication ID. No static release blocker was
found. Its canonical API contract now passes all 53 selected HTTP cases on
the frozen source and corrected disposable test setup, with matching runner
manifests. Eligibility reflects the current configured time-zone date; changing
the form's taken-at date can change schedule or stock eligibility, and the
direct POST remains the final authority. The new HTTP contract has an observed
field-missing RED before implementation and a production-green run afterward.

The schedule form no longer sends `dose_amount` or `dose_unit` to the direct
POST. The API therefore applies the effective schedule configuration for the
selected taken-at date, including taper steps. An exact replay with omitted
fields compares the original stored take and does not derive a new dose. The
form describes its schedule dose as calculated for the selected time; it does
not display an unsupported base amount. Person-medication forms retain their
explicit amount and unit. A numerical preview of the effective schedule dose
at an edited taken-at time remains a UI parity gap; this review does not claim
that preview is present. The added taper browser case is structured to prove a
today dose of 0.75 ml from a schedule with a 2.25 ml base: it checks the new
history row and stock moving from 10 to 9.25 ml. Its yesterday 1.5 ml step is
fixture context, not a submitted browser dose, so that step is not covered by
this case. The case passed in the final dedicated browser run.

An eligible alternate stock item can be chosen for a source medication. After
that write, the redirect returns to the source medication detail; its displayed
stock correctly stays unchanged, while the consumed alternate stock is visible
when the picker is reopened. The generic success notice does not identify the
consumed item or link to its refreshed stock. This is a remaining alternate
stock confirmation gap for full UI parity, not a demonstrated wrong write in
the bounded first-dose journey.

The intermediate UI v1 browser selection passed its five original journeys,
including Escape focus restoration. I independently inspected the saved
390×844 mobile dose dialog and stock screenshots plus the desktop history
screenshot: the mobile form controls and Log button fit, the resulting stock
shows 18.75 ml, and the desktop dashboard includes a new 1.25 ml history row.
These screenshots predate the URL/source and mixed-credential corrections and
do not constitute final browser acceptance or full Rails visual parity.

## Final bounded verdict

Requirements and code quality pass for the bounded first medication journey.
The canonical joint run passed 53/53 HTTP, 7/7 login-browser and 7/7
medication-browser cases, with no skips, matching pre/post runner manifests and
completed owner-scoped cleanup. The earlier isolated browser RED proved the
missing URL/source guard while its five original cases remained green; the
taper case was red before the schedule override fix. Final mobile dose controls
and the Log button fit the saved screenshot, and desktop history shows the
new dose. This is first-journey acceptance, not full Rails UI parity. The
effective-dose preview and alternate-stock confirmation described above remain
follow-up work, and the capability list is a present-time hint rather than a
guarantee for an edited taken-at time.
