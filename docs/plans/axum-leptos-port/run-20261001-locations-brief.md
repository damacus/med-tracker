# Locations brief

Owner: locations seat. Scope: authorised list/detail/add/edit and permitted medication contents. No deletion or membership administration.

Owned production paths: `rust/api/src/web_pages/locations.rs`, `rust/web/src/locations.rs`. Root owns route registration, shared helpers, capability API exposure, styles, translations and manifests.

Routes: GET/POST `/households/{slug}/locations`; GET `/households/{slug}/locations/new`; GET `/households/{slug}/locations/{id}`; GET/POST `/households/{slug}/locations/{id}/edit`.

Use cookie-only WebApi session authentication and slug lookup; native named name/description controls preserve string drafts in SSR. POST validates Origin and authenticity_token and forwards API CSRF. The API remains authoritative for access and field validation. Success redirects to the saved location; contents read from the authorised medication API and filter by location ID. Medication author owns fresh location-selector reads.

API representations carry numeric location IDs; paths also accept portable IDs. Editing requires API ETag/If-Match; shared WebApi must expose ETag and conditional/idempotent dispatch. Action links require authoritative API capabilities, not independently inferred membership roles. Requested root helpers: household page shell, native Field/FormShell, field errors and private redirects.

Context7 Leptos book confirmed native named input attributes and textarea child text initialise SSR values. Serena initial instructions loaded; activated language server is Ruby, so Rust inspection uses normal repository reads.

Production implementation waits for a recorded failing route/API/browser case from test owner or root. Acceptance requires persistence/reload, invalid draft retention, foreign access denial, CSRF rejection, permitted contents and immediate medication selector usability. Desktop/mobile browser evidence and translations depend on shared integration.
