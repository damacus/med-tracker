# Locations report

Scope: location list/detail/add/edit with permitted medication contents. No deletion or membership administration.

Changed paths: `rust/web/src/locations.rs`, `rust/api/src/web_pages/locations.rs`, this report and the locations brief.

RED: test owner recorded 08:46 UTC `CARGO_NET_OFFLINE=true rtk task -d rust/web test TEST_FILE=household_rendering`, Cargo 101 / task 201, missing `medtracker_web::locations` import. The observable rendering test requires native POST/CSRF, retained name/description, associated errors and ETag draft.

Second RED: focused renderer initially failed escaped description assertion (`Retained &lt;description&gt;`). Leptos textarea emits raw child text; renderer now explicitly escapes ampersand and angle brackets before encoded textarea content.

Runtime RED: test owner and coordinator confirmed authenticated `/locations` returned 404 rather than 200 on immutable product digest `d78ff035...`; raw evidence `/tmp/medtracker-runner-route-red-browser-raw.log`, task exit 201. API page implementation began after that evidence.

GREEN: `rtk task -d rust/web test TEST_FILE=household_rendering` returned 0, 6 tests passed. Test owner subsequently strengthened malicious textarea coverage and recorded 8/8 GREEN. `rtk task api:fmt-file FILE=rust/web/src/locations.rs` and `rtk task api:fmt-file FILE=rust/api/src/web_pages/locations.rs` returned 0. Only cache-write warnings and an existing dependency future-compatibility warning appeared.

Rendering includes native named fields, SSR drafts, CSRF/ETag/idempotency hidden controls, field-associated errors, API capability props, explicit locale and canonical labels. Unrecognised API validation messages remain escaped upstream text. API handlers dispatch reads/mutations through shared cookie-only WebApi, check CSRF/Origin, forward ETag and idempotency headers, preserve 422/409/428 drafts and show only authorised medication API contents. Successful writes redirect to a reloaded detail page.

Review fixes: removed user-controlled `saved` query success notices; location readback alone does not prove the requested write happened. Redirects now contain only the saved location path. Test owner verified Rails literal new-page H1 `New Location`; new/invalid-create rendering uses coordinator-approved canonical `forms.locations.new_title` to match it while allowing all five locales.

Integration check: `rtk task api:check` returned task 201 / Cargo 101 for a medication module redirect argument mismatch, with no reported Locations errors. Medication owner and coordinator received the finding; combined compile GREEN is pending.

Unverified: API route integration, create/edit persistence, foreign records, invalid CSRF, mutation replay/conflict, immediate medication selector, permitted contents, desktop/mobile UI and all-locale browser acceptance. The renderer tests do not establish those journeys.
