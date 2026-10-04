# Design

## Reference and route

Use the current Rails `/households/{slug}/profile` page as the reference for copy, ordering, interaction and responsive layout. The canonical Rust route uses the same path. A validated `section` query selects Profile, Security, Notifications or Advanced. `/households/{slug}/settings` redirects to Profile for old links. Every household slug is encoded as a path segment.

The shared hero and tab strip stay on all sections. Only the selected section is loaded from its backend so account secrets, recovery codes and one-time API tokens do not leak into hidden panels. Tabs support arrow, Home and End keys with focus restored after server navigation. The desktop sidebar and mobile top and bottom navigation lead to the same Profile destination.

## Profile

Show the signed-in person's information and the four Rails setting rows. Open time zone in a modal and photo, shortcuts and appearance in sheets. Save time zone, Gravatar opt-in and shortcuts through the existing profile API with browser CSRF, trusted-origin and manage-grant checks. Read back successful writes. Use the authenticated avatar route for uploaded images, then the opt-in Gravatar choice and initials fallback. The external Gravatar image requires explicit authorisation because its URL contains an email-derived hash; until authorised, the page CSP must not allow that external request.

Rails stores a time-zone label such as `London` in account preferences. The Rust browser offers the 152 labels in ActiveSupport 8.1.3's `TimeZone::MAPPING` and retains an existing IANA identifier as the selected option. A shared translation resolves either form to IANA for validation and dashboard calendar calculations while the API returns the original stored label unchanged. The checked-in map is copied from the installed ActiveSupport 8.1.3 source so this migration does not depend on a Rails runtime inside the Rust service.

Appearance uses the same local font families and stored theme/mode choices as Rails. Serve the font files locally. Apply the preference on Profile, other Rust household pages, and the sign-in and factor pages. The modal and sheets must trap focus and return it to the opener.

## Security

Match the Rails email, password, authenticator, recovery-code and passkey controls. Email changes remain pending until a one-use verification link reaches the new address. Password changes verify the current password. Authenticator enrolment verifies a fresh code and password before activation. Recovery and passkey operations require the intended account and fresh proof. Password sign-in with an enrolled factor completes through a short-lived, purpose-separated pending session; a factor or recovery code is consumed once before a normal browser session exists.

## Notifications

Integrate the separately implemented notification preferences with browser push enrolment, managed-person selection and reminder times from Rails. Preserve unrelated preference fields on each save. Browser push uses the existing subscription API and a same-origin script. A person with view-only access sees read-only settings, while every write is checked again at the API boundary.

## Advanced

Implement API token creation and revocation, bounded health JSON and backup ZIP downloads, experiment choices, system information and account closure. Show a new token only once in the full Profile shell. Validate all downloads before decoding or streaming. Verify the current password inside the closure transaction and clear the browser session after closure. Describe retained household health history accurately.

## Verification

Write an observable failing test for each changed behaviour before its implementation. Run the applicable Rust Task format, lint, API, contract and browser gates. Exercise all four sections through a disposable authenticated browser fixture at desktop and mobile sizes, including denied writes, CSRF failures, reload persistence, keyboard and dialog focus, and the computed font. Save screenshots under `docs/screenshots/`. Keep Rails and notification PR histories untouched.
