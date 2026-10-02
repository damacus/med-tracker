# localisation report

Status: renderer, locale structure and real five-locale browser evidence GREEN;
final independent programme audit in progress.

Baseline: eccf62aace3aab81bf871b380c964ff0ee28e7b6.

Original gap: People and Locations renderers displayed unknown API errors verbatim.
People's API adapter already translates known validation errors before rendering;
the renderer must recognise those values rather than replacing useful errors.
Existing form inputs retain drafts and have labels and associated field errors.
Real browser evidence now covers authorised list/detail/add/edit, persistence,
invalid drafts, keyboard navigation and desktop/mobile width in all five locales.

Accepted bounded fix: preserve `Text::api_error`'s existing Option contract; add a
fallible form-error adapter that translates known English messages, preserves
their already-translated equivalents, and uses `errors.messages.form_invalid`
for everything else. English: "This value could not be saved."
Equivalent nodes were added to en/cy/ga/es/pt. No clinical advice, permission
change, validation relaxation, raw upstream detail or promised retry outcome.

RED requirement: both renderers use the safe generic text for an unknown field or
base error in all five locales; recognised English and pretranslated known errors
remain useful and field-associated. Test ownership remains with the test writer.

Coordinator and independent policy reviewer accepted this bounded adapter policy.
Success means persisted detail and reloaded list evidence; a new flash interface
is outside A. People has no notice input and Locations' API passes an empty
notice. Those are optional follow-up observations, not acceptance defects.

RED evidence: `evidence/A-RED-001.log`, unchanged baseline, 34-path pre/post
manifest identical. `unknown_people_and_location_errors_use_safe_localised_messages_and_retain_drafts`
failed at line 48 because People/en exposed the diagnostic; known/pretranslated
preservation passed. Implementation followed Luna's explicit source-release
message. The coordinator then found the immutable copy had not been confirmed;
the C manifest was superseded and recaptured with these valid post-RED edits.
Actual C source copy subsequently validated at `tmp/contract-tests/run.moUPe9/source`,
digest prefix `422566e6`, with 295 copied input hashes matched. Coordinator
released the product capture hold before focused GREEN.

Implemented: new `Text::form_error` keeps the existing `api_error` Option contract,
recognises its seven English messages and exact current-locale equivalents, and
returns the translated generic for everything else. People and Locations field
and base errors use the fallible adapter. Added only `errors.messages.form_invalid`
to en/cy/ga/es/pt. Existing input rendering and field associations are preserved.

Changed product files: `rust/web/src/{household_i18n,people,locations}.rs` and
`config/locales/{en,cy,ga,es,pt}.yml`. No tests, comments, shared wiring, Git or
API adapters changed by this owner. No further product edits followed GREEN.

Independent static review: safe-adapter requirements and quality/security PASS.
Focused GREEN: Luna `A-GREEN-001` passed the new renderer suite (2/2) and legacy
`household_i18n` suite (11/11). The 37-path pre/post digest prefix `efcfa665`
remained unchanged. No source correction was needed after GREEN.

Locale structure: `evidence/FINAL-TOOLING-001-locale.log` confirms the translate
skill's existing tree checker passed across en/cy/ga/es/pt through the coordinator's
Task wrapper. All five catalogue trees match.

Real browser evidence: `evidence/ABC-BROWSER-GREEN-004-api-final-full.log` passed
35/35 across seven files. Its ten People/Locations journeys cover en/cy/ga/es/pt
at 1400x900 desktop and 390x844 mobile, with real login and locale cookies.
They verify authorised list/detail/add/edit, keyboard Enter submission and Tab
order, API read-back after create/edit, translated associated blank-field errors,
retained invalid drafts, unchanged persisted records after 422, escaped names and
textarea content, no-store responses, correct document language and no overflow.

Snapshot evidence supplied by Luna/coordinator: 361-path pre/post manifest matches
at digest prefix `4fea2556`; copied source prefix `52eca56`; disposable fixture
prefix `d4b7`. Forty fresh combined-run screenshots are under
`docs/screenshots/ABC-BROWSER-GREEN-004-52eca56`; A's invalid-person/location
screenshots are named by locale and viewport.

Limits: unknown diagnostic suppression is proved by the pure renderer tests;
the real browser cases exercise recognised blank-field validation. A's browser
matrix proves the authorised creator's journeys, not every actor's permissions.
Final independent programme audit and coordinator publication remain outstanding;
this owner makes no programme-completion, commit or push claim.
