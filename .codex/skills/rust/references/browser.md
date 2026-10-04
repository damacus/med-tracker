# Browser UI

Use the current SSR route and rendering conventions. Inspect existing native
controls and Leptodon integration before adding another form library or client
runtime. Keep authorised data projection outside Leptos components.

A management journey includes its entry link, form, validation failure,
successful persistence and useful return destination. Preserve submitted values
and associated field errors; verify the saved record can immediately be used
in the next step. Do not infer completion from rendered markup alone.

Reuse the locale source and adapter used by neighbouring pages. Keep locale keys
structurally aligned across `config/locales/`; use the translation skill for
locale edits. Test locale selection and interpolation at the browser boundary
when affected, including errors and success messages.

For visible changes, exercise keyboard navigation and desktop/mobile layouts
in the existing Playwright harness. Save evidence under `docs/screenshots/`.
Screenshots supplement assertions about submission, validation and navigation.

PWA caching stays limited to public assets. Authenticated HTML, API responses
and medical images must not become offline patient records. Verify logout,
expiry and offline transitions when changing that boundary.
