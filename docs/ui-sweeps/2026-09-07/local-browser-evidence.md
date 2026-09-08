# Local browser evidence

Baseline: `45db2112`; local development at `http://localhost:58587`, seeded fixture household,
`admin@example.com`. Canary was not used. This is initial evidence, not final acceptance.

## Runtime

Docker Desktop started successfully. Fresh image builds repeatedly timed out obtaining
`rubylang/ruby:4.0.6-resolute` metadata. A temporary Compose overlay uses cached test/development
runtime images with the current checkout bind-mounted; the entrypoint checks locked dependencies.
`task test:preflight` passed: 31 examples, zero failures.

The cached image lacked Chromium revision 1234. Installed the locked Playwright browser into ignored
`tmp/ui-sweep-browsers` using the repository internal task wrapper via a temporary Taskfile.
The overlay sets `PLAYWRIGHT_BROWSERS_PATH=/app/tmp/ui-sweep-browsers` for tests.
Commands use `env COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml task ...`.
This does not validate a fresh Docker image build.

Runtime check through the same wrapper returned Ruby 4.0.6 (`03b6d3f889`, aarch64-linux).
Cached test image: `sha256:acfb49eea558cb574b30be28708ce1ecdb3efe2858d278b697c3c3441ede64ef`.
Cached development image: `sha256:7a95039e401913b005b78d5f086a00fda1a1a783c7b47aafc1b525d400df6b1b`.

## Confirmed initial defects

| ID | Route/state | Evidence | Expected |
| --- | --- | --- | --- |
| INV-03 | Home location show, admin, 390px dark | Tab from Add member focuses Remove member; computed button opacity is 0; ancestors opacity 1. Screenshot shows invisible focused control. | Focused action visible and discoverable on touch. |
| INV-04 | Profile, 320px dark | Notifications button bounds 160–228.5; text bounds 156.98–231.52; client width 69, scroll width 72. Labels crowd adjacent tabs. | Text contained in usable tab controls. |
| ADM-05 | Admin dashboard, fixture owner | Both dm+d import links navigate to dashboard with “You are not authorized to perform this action.” | Do not advertise unavailable actions; preserve backend policy. |
| AUTH-01 | Sign out to login, 1280px light | “You have been logged out” appears in both global notice and inline login feedback. Source review confirms both are visible alerts. | One visible and announced message. |
| AUTH-02 | Verification resend, 1280px light | Submit says “Resend Verify Account Information”; footer combines “Back to Login” with a “Welcome back” link. | Clear resend action and a single return-to-sign-in link. |

## Route smoke

The following rendered at 390/1280 in light theme with zero page horizontal overflow: dashboard,
inventory, locations, people, finder, medicine reviews, reports, admin dashboard, admin people,
admin users, invitations, household settings and audit trail. The eight main navigation destinations
also passed this check at 1280 dark; all listed destinations passed at 390 dark.

Import links redirected to unauthorised rather than the intended page and are not counted as passing.
This page-level check alone does not establish component containment, full accessibility or every
form/error state. Focused tests and the remaining audit matrix provide that evidence separately.

Login, password reset request and verification resend render without horizontal page overflow at
390/1280 in light mode. Account recovery was not submitted; no emails or credentials were changed.

The populated All Family dashboard was inspected at 320px in light mode. Ordinary fixture timeline
rows keep their Give action inside the card and wrap “Movicol Paediatric Plain” cleanly. This is not
evidence for unusually long data; the long-name timeline remains a reproduction candidate.

## Additional fixture journeys

The parent fixture at 320px offers only Parent Person, Child User and All Family in the dashboard
selector. Its people list contains only parent and child and has no page overflow. The nurse fixture
dashboard and people list have no settled page overflow at 390/1280; the sidebar identifies this
account as Member, so its name alone is not proof of clinician-role coverage.

The adult-patient fixture at 390px offers only itself and All Family in the dashboard selector and
only itself in the people list, without page overflow. During this journey after the auth-flash fix,
logging out produced one visible “You have been logged out” message; the DOM text count was also 1.

Selecting Child User exposed a separate misleading dashboard state: Next Due is “Now”, Due Now is
1 and Tasks Left is 1, while the page says “Routine tasks done today” and “No routine tasks due
today”. The only available medicine is under As needed. Independent source review confirms that the
headline aggregates as-needed availability with routine tasks. Tracked separately in
[#2125](https://github.com/damacus/med-tracker/issues/2125), because choosing the meaning of these
headline totals changes a presentation contract already codified by existing tests. It remains
unfixed by this sweep; as-needed availability and dosing rules are unchanged.
