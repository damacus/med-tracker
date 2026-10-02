# People report

Owned paths:

- `rust/web/src/people.rs`
- `rust/api/src/web_pages/people.rs`
- this report and the People brief

RED recorded by test owner at 08:46 UTC:
`CARGO_NET_OFFLINE=true rtk task -d rust/web test TEST_FILE=household_rendering`
failed with Cargo 101/task 201, E0432 unresolved `medtracker_web::people` at
`household_rendering.rs:3`. The test checks native edit POST, initial string
values, escaping, CSRF, associated errors and exact Rails submit label.

GREEN check:
`rtk proxy env CARGO_NET_OFFLINE=true task -d rust/web test TEST_FILE=household_rendering`
compiled the renderer and passed
`person_edit_renders_initial_values_native_submission_and_associated_errors`.
Combined test target returned task 201: four tests passed and two other owners'
textarea draft tests failed. The People test itself is green.

After other owners fixed their textarea rendering, the same command returned
exit 0 at 08:49 UTC: all six household rendering tests passed. This check also
includes encoded People form/link slug paths without double encoding the shell.

Test owner subsequently recorded eight passing renderer tests, including
selected adult/minor/dependent-adult values, true/false capacity checkbox state,
and all five literal Rails edit headings with draft retention and error links.

Locale-error review RED: renderer locale loop failed Welsh canonical blank
message. Renderer now maps canonical API validation messages before summary
and field output. Same household renderer command then passed all People
cases, including exact blank errors across all five locales. Combined target
was eight pass/one fail for a coordinator-owned medication catalogue key.

Renderer implements list/detail/form with native inputs and checked/selected
SSR values, catalogue labels, and associated field messages. Person types and
capacity use the existing API values.

The browser adapter was written after the coordinator authorised implementation
from immutable runtime route RED. Authenticated `/people` returned 404 instead
of 200; source digest was
`d78ff0355198b24dce59b8792116af8d7df001ae8688b6b40ad54b4267979cbd`.
Evidence: `/tmp/medtracker-runner-route-red-browser-raw.log`, task exit 201.

Adapter routes list/detail/new/edit and native create/update. It uses shared
locale and capability helpers, exact API permissions, actual origin and CSRF,
internal writes, private validation responses, persisted detail read-back and
303 redirect with renewal cookie. API capability and router registration are
coordinator-owned; browser acceptance remains pending.

Integrated API compile GREEN:
`rtk proxy env CARGO_NET_OFFLINE=true task api:check` returned exit 0 after
shared helpers and routes were registered. Successful saves now use the shared
redirect helper, preserving 303/no-store and session renewal cookies.

Web Clippy GREEN:
`rtk proxy env CARGO_NET_OFFLINE=true task -d rust/web lint` returned exit 0
with all targets and warnings denied.

No acceptance claim yet. Permissions, tenant isolation, persisted mutations,
desktop/mobile screenshots and non-English browser journeys are unverified.
