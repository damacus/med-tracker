# Medication journey browser contract

The shared browser suite is `rust/web/tests/medication-journey.test.mjs`. It uses a disposable contract fixture and `BASE_URL`, so the same user actions can run against Rails and the Rust web implementation. Rails routes, labels and visible outcomes were taken from `config/routes.rb`, the medication and dashboard Phlex components, and the Rails dose system specs at baseline `2a8b4c1b`.

## Covered journey

- Desktop (1400 × 900) and mobile (390 × 844) each sign in with the owner account, follow the first authorised household redirect, select their own medication from inventory, and see 20 ml on its profile.
- The user opens **Log**, chooses the as-needed assignment, confirms the person, 1.25 ml dose and stock source, and can dismiss **Record dose** with Escape. A separate strict test requires focus to return to the visible medication-page **Log** link after dismissal at both viewports; a failure records the active element and a screenshot.
- A malformed dose submitted through the browser form shows the Rails validation error and leaves stock at 20 ml. A subsequent valid keyboard submission shows the success notice, 18.75 ml stock, and exactly one matching dose in **Previous Doses Today**.
- A bad password stays on login with an alert. The signed-in owner cannot open a foreign household medication by ID.
- The suite checks horizontal fit at both viewports and writes dialog, stock and history screenshots when `SCREENSHOT_DIR` is set.

## Fixture and selector contract

The fixture provides `primary_email`, `household_slug`, two isolated browser records (`journey_browser_desktop_*` and `journey_browser_mobile_*` medication ID/name and assignment ID), plus `journey_browser_foreign_medication_id/name`. Each writable record has an as-needed 1.25 ml assignment, 20.0 ml tracked stock and no earlier take. Separate records prevent one viewport from changing the other's starting stock.

The suite uses visible roles and labels for login, medication, dialogs, dose, success, stock and history. It uses existing Rails `data-testid` values only to distinguish the exact as-needed administration trigger and today's history section. It does not require a household chooser: Rails currently redirects the owner to the first active household dashboard after login.

## Next web implementation requirements

The captured Rails desktop and mobile dialog screenshots show the same clear order: **Record dose** title and explanation; person, medication and **1.25 ml** context; labelled **Taken at** field; named stock source; then a primary **Log** action. At 390 px the content stacks within the screen, the action remains reachable, and there is no horizontal page overflow. Rust should retain that information and keyboard path while using the shared API write; it should render the returned error without losing the entered values or changing the form's client UUID.

After a successful write, the medication profile must visibly show **18.75 ml remaining** for that record, and the selected person's dashboard must show one **Previous Doses Today** entry naming the medication and **1.25 ml**. The mobile screenshot shows the profile content extends below the first viewport, so stock may require normal scrolling; it must remain in the page and be reachable. Escape from the nested dose dialog must restore focus to the still-visible medication-page **Log** link. Rails currently focuses `BODY` there; that existing defect is recorded by the strict test and must not be copied into Rust.

## Verification state

The final isolated Rails browser baseline completed with three passing cases and two failing strict Escape focus cases. The desktop and mobile owner journeys each recorded one 1.25 ml dose, showed stock falling from 20 to 18.75 ml, and found exactly one matching history row for the selected managed person. Bad credentials and foreign medication access were rejected. On both viewports, Escape from the nested dose dialog left `BODY` focused rather than the visible medication-page **Log** link. This is a Rails accessibility defect; the strict assertion remains active for Rust. The final mobile stock screenshot visibly shows 18.75 ml. The mobile history screenshot captures the dashboard's first viewport rather than its below-fold history row; the row is verified by browser assertions, not that screenshot. No Rust result is claimed.

The dedicated Rust browser target now scrolls the asserted **Previous Doses Today** section into view before its screenshot. This is an artifact improvement only; the frozen Rails baseline and the history assertions need no repeat run for it.

## Dedicated Rust RED

`task api:browser-rust` provisions the disposable contract fixture, starts the isolated Rust API, and runs only `medication-journey.test.mjs` in the existing Playwright sidecar. Its screenshots go to `docs/screenshots/journey-medication-rust`; the canonical login smoke selection remains separate. `task api:contract-runner-test` passed fake dispatch, failure propagation and cleanup; `git diff --check` passed.

The first live Rust run (`mtcontract-232df6665f68446e`, log `/Users/damacus/Library/Application Support/rtk/tee/1790350900_task_api_db4caa.log`) produced a genuine red: one of five cases passed, four failed, none skipped. Invalid credentials and foreign medication access passed. At desktop and mobile sizes, the journey cases timed out waiting for the exact fixture medication heading on the inventory list (H2); the separate focus cases timed out waiting for the detail heading (H1). None reached the dose dialog, so this run does not assess Rust focus restoration, dose recording, stock or history. The product UI must supply those routes and visible states before the next browser pass.

## Final baseline adjustments

Two isolated browser medications share one person, so today's dashboard legitimately contains both 1.25 ml entries after the desktop and mobile tests. The history assertion scopes the amount to the row containing that test's exact medication name while still requiring the name exactly once. The isolated runner also enables Rack Attack: repeated `/login` POST requests can return 429 at the five-in-20-seconds threshold. The browser helper captures the POST result and retries once only for an observed 429, respecting its bounded `Retry-After` and loading a fresh login form. The bad-password check remains unchanged.
