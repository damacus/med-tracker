# First-party medication journey product report

## Implemented boundary

- `rust/api/src/lib.rs` routes shared people, schedules, and person-medication reads through the existing API middleware. `rust/api/src/oauth.rs` lets the existing household-list API use a validated browser session while retaining explicit bearer precedence.
- `rust/api/src/web_pages.rs` serves the household medication list, detail, dashboard history, dose form, and static assets. It resolves the household slug and all clinical data through the shared API router. It releases its session transaction before dispatching internal API requests, forwards the browser's cookie and actual Origin/Referer, and carries API session-cookie renewal to HTML responses. HTML pages reject an explicit Authorization header before cookie lookup.
- `rust/web/src/lib.rs`, `medication.js`, and `medication.css` render responsive Leptos views and accessible native dialogs. Current API `can_record` and ordered eligible-stock IDs determine Log visibility and stock choices. The shared dose POST remains the final authorization and stock authority.
- The form validates origin and session CSRF, checks that the submitted source belongs to the route medication through shared API shows, and retains one UUID across rejected attempts. Successful API writes redirect to a fresh detail form with a new UUID; the success notice appears only after the redirected page verifies the take through the shared API. Rejected forms retain source, time, stock, UUID, and the API's 422/403/409 status.
- Schedule dose forms send source, time, and selected stock without an amount/unit override. The API resolves the effective taper step for the chosen time. The UI labels the schedule dose as calculated for that time; person-medication forms continue to show and submit their configured amount/unit.

## Evidence and current limit

The original first-party journey browser cases passed 5/5 on desktop and mobile in the first isolated run. A later companion browser case established RED for an internally valid source B posted through medication A's form: it returned 303 where 403 was expected. The route/source guard was added after that result.

The [final frozen Rust candidate](journey-web-journey-runner-report.md#final-frozen-rust-candidate) passed 53/53 canonical HTTP tests, 7/7 login browser tests, and 7/7 medication browser tests with no skips. The medication run covered CSRF denial and UUID replay, the source/route binding, desktop and mobile keyboard flows, and a taper dose recorded at the current effective 0.75 ml step with stock reduced from 10 to 9.25 ml. Full and scoped pre/post source manifests matched, and six final desktop/mobile screenshots were copied to `docs/screenshots/journey-medication-rust/`. These results verify the bounded first-party journey; they do not establish full Rails UI parity.

Local checks pass: `task api:fmt:write`, `task api:clippy`, `task api:test` (9/9 unit tests), the Rust web Taskfile `test` (2/2 unit tests), and `git diff --check`. Focused units demonstrated RED then GREEN for configured-timezone Today filtering, rejected-form HTTP status, and mixed Authorization rejection.

## Follow-up UI gaps

- Schedule dose preview intentionally gives no numeric amount because the effective taper dose depends on the selected time; the recorded amount and stock are verified by the shared API. A numeric preview would need an API-owned effective-dose representation.
- An eligible alternate stock row can be selected and decremented, but the source medication detail returns to its own truthful inventory figure after success. The confirmation does not yet name or link to the alternate stock row and its updated balance.
