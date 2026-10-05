# Migration input and capability inventory

The machine record `source-inventory.json` pins the observed checkout revision,
source hashes, Rails route declarations and job paths. It includes local relocation
changes; its revision is the parent commit, not a claim that those changes were
already committed. `task inventory:check` rejects changed or added source inputs.
Regeneration requires reconciling this inventory with the changed behaviour.

## Source owners and journeys

| Capability | Source and observable coverage | Migration acceptance |
| --- | --- | --- |
| Household roles, invitations, person grants and capacity | `rails/app/policies/`, `rails/app/models/person.rb`, `rails/config/routes.rb`; `rust/contract-tests/tests/care.rs`, `invitations.rs`, `household_minor_readiness.rs` | Preserve adult=0, minor=1, dependent_adult=2 and capacity false for latter two; admin/clinician/self/carer/parent/denied journeys |
| Medicines, dosage options, stock removal/adjustment/order/receipt | `rails/app/services/adjust_medication_inventory_service.rb`, `rails/app/controllers/api/v1/`; contract `medication_stock.rs`, `dosage_health.rs`, `household_stock_permissions.rs` | Full writes, stock/audit transactions and conflict behaviour |
| Schedules, doses, pause/resume and missed occurrences | `rails/app/services/medication_administration/`; contract `doses.rs`, `schedules.rs`, `household_treatment_permissions.rs` | Concurrent requests and replay must preserve stock and audit exactly once |
| Sync, offline capture and replay | `rails/app/services/offline_dose_eligibility.rb`, `rails/app/controllers/api/v1/sync/`, `rails/app/javascript/`; contract `sync.rs`, `replay.rs` | Explicit browser offline outbox, authorization and idempotent batches |
| Profile, avatar, API tokens, notification preferences | `rails/config/routes.rb` profile routes, `rails/app/controllers/profiles/`; contract `profile.rs`, `web_profile.rs`, `devices.rs`, `push_delivery.rs` | Full profile journey, session/device revocation and preference persistence |
| Lookup, NHS dm+d import, AI suggestions and medication reviews | `rails/app/services/medication_reviews/`, `rails/app/jobs/nhs_dmd_import_job.rb`; contract `lookup.rs`, `reviews.rs`, `nhs_dmd_import_api.rs` | Authorization, job reconciliation, source provenance and failure responses |
| Portable import/export, PDFs and health history | `rails/app/services/portable_data/`, `rails/app/services/reports/`; contract `portability.rs`, `reports.rs` | Signed downloads, encryption formats, retention, fonts and timezone boundaries |
| Native API and pinned clients | authoritative `docs/api/openapi.v1.yaml`, `client-tools/openapi-generator/`, `mobile/android/`; contract `openapi_*.rs` | Preserve routes, request/response envelopes and pinned client checks |
| FHIR R4 and SMART | `rails/config/routes.rb` `/api/fhir` resources/discovery, `rails/app/misc/rodauth_main.rb`; contract `fhir.rs`, `smart_fhir.rs` | Real authorization server, scopes, PKCE, discovery, audience and negative access |
| MCP | `rails/config/routes.rb` mounted `MedTrackerMcp::RackApp`, `rails/lib/`; contract `mcp.rs` | Maintained MCP server transport and same authorized domain operations |
| Signed attachments | Rails ActiveStorage routes and household policies; contract `uploads.rs`, `envelopes.rs` | Signing/key compatibility, expiry, private blobs and representations |
| Platform/admin/support/household lifecycle | `rails/config/routes.rb` platform/admin namespaces, `rails/app/policies/support_access_session_policy.rb`, lifecycle tasks | RLS actor context, expiry, owner promotion and exports/retention/restore evidence |

These pointers identify existing behaviour and tests, not completion evidence for
Loco. Legacy Axum/Leptos remains input only and does not implement root routes.

## Authentication and stored formats

Rails uses Rodauth, `rodauth-oauth`, bcrypt and WebAuthn. The owning configuration is
`rails/app/misc/rodauth_main.rb`: PKCE, token revocation, rotating refresh tokens,
hashed token columns, `password_hash` and bcrypt's 72-byte limit must survive the
migration. No pepper was observed in the inspected password configuration; verify
all environments before concluding there is none. Rails and Rust session formats
are distinct. The Rust OAuth implementation uses private `cookie::Key` claims and
SHA-256 URL-safe session digests. TOTP/encrypted columns, token formats, attachment
signatures and portable encrypted exports require explicit compatibility fixtures
before any replacement; successful fresh login does not prove compatibility.

## Jobs and queue prerequisite

The machine record enumerates every Rails job. Current jobs cover NHS dm+d import
and reconciliation, reminder scheduling/delivery, missed-dose and low-stock
notifications, medication-review refresh, support-access/export expiry and an
observability canary. `rails/config/recurring.yml` is the schedule owner.

Loco 1.2 provides `worker` and PostgreSQL `BackgroundQueue`, including database
queue storage. The foundation enables that standard path in a disposable database.
Queue startup can create its table: least-privilege provisioning must be designed
before the application role is adopted. Runtime retry/reaper, schedules, shutdown,
idempotency and pooled RLS behaviour remain worker-tranche acceptance, not assumed
from compilation. No second queue dependency has been introduced.

Loco JWT authentication and OAuth client initialization do not replace Rodauth's
SMART authorization server. A maintained authorization-server implementation and
SMART/FHIR interoperability evidence are a prerequisite for tranches 2 and 3.
WebAuthn and Web Push must use maintained libraries. No custom security protocol
has been added in this foundation.

## Browser themes and assets

`rails/app/javascript/appearance_boot.js` and
`rails/app/javascript/controllers/appearance_controller.js` own the
`med-tracker-theme` and `med-tracker-appearance` storage keys. Preserve
Light/Dark/System, live OS changes and palette names. Existing CSS, local licensed
fonts, PWA/service-worker assets and all interaction tests are hashed inputs.
Tranche 4 must export CSS from the official daisyUI theme creator and verify
keyboard tabs/dialogs, focus, validation, offline replay and desktop/mobile
screenshots. The Tera foundation page is not that completed browser migration.

## PR salvage dispositions

| PR | Captured source | Disposition |
| --- | --- | --- |
| 2419 schema | `6d607c71c393a79cd259e39f13d48f1977fba1e2` | Retain 177-migration schema work for tranche 2 adoption checks |
| 2418 scratch | `d11483c2dcfaa5e3cdbcaaab8f3f5707512efc0b` | Retain runtime assets/CA/timezone/font lessons for tranche 6 |
| 2389 notifications | `ad7502b8cd2df9f3a0e6ec46edc55956dc8d61d9` | Retain all five switches for API/browser/worker acceptance |
| 2390 profile | `4355b2fd47a37e89c0775dbf588de5edca8ceabb` | Retain complete four-tab profile behaviour |
| 2395 fonts | `1b60dcbf4ab4c60b3f4be1798d939fbe00fd11c4` | Retain official Geist 1.7.2 and licenses |
| 2397 styles | `750f347589e4a45f70fa089d66bba546a7046a05` | Closed framework choice; palette/geometry/contrast source retained for tranche 4 |
| 2399 UI infrastructure | `73f3dfa8617728274974e8a37f240eafa5dbdea9` | Closed framework choice; useful behaviour/assets retained pending actual replacement evidence |
| 2402 UI infrastructure | `8bc8e335ac2e0a531182698711f8d4f8b0ce5538` | Closed framework choice; useful behaviour/assets retained pending actual replacement evidence |
| 2403 UI infrastructure | `ac6cdc99d7e92e20433e78d405b94ed5e8a17fa6` | Closed framework choice; useful behaviour/assets retained pending actual replacement evidence |

PRs 2397, 2399, 2402 and 2403 are closed as superseded framework choices at the
user's explicit request. The coordinator verified their current heads matched
these snapshots before closing; source branches were not deleted. Their useful
behaviour, themes and assets remain migration requirements and are not claimed
replaced by the foundation page. PR 2381 is outside this migration. All nine snapshots
are retained as `refs/loco-migration/input-pr-<number>` independently of remote
branch pruning; see coordinator `pr-inputs.md` for those mappings.

Foundation commit `f54630d41e1388a5fd552d9de385ea78995b7653` is published on
`codex/loco-migration` in [draft PR 2451](https://github.com/damacus/med-tracker/pull/2451).
[Issue 2450](https://github.com/damacus/med-tracker/issues/2450) tracks remaining
work. Independent source review and full migration acceptance remain pending.
