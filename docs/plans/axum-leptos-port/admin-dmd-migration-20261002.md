# Admin area and NHS dm+d migration plan

Status: in progress, 2 October 2026

Scope: migrate the Rails `/admin` surface to the Axum + server-rendered
web stack (`rust/api/src/web_pages/`). The NHS dm+d release import is the
first deliverable — it is broken in production Rails and will be **fixed by
replacement** in the Rust UI, not repaired in Rails.

Decision (2 October): no Rails-side fix for the dm+d import. The Rails
importer was verified working end-to-end from the CLI
(`dev:import-dmd-release`, release `nhsbsa_dmd_9.3.0_20260928000001.zip`,
84,410 imported / 16,942 skipped); the production stall comes from the
job/broadcast delivery path, which the Rust port replaces wholesale.

## dm+d import (first slice, this branch)

New `rust/api/src/nhs_dmd/` module plus an admin web page.

- **API** — `GET`/`POST` `/api/v1/households/{household_id}/admin/nhs_dmd_imports`
  and `GET .../{id}`. Household-scoped URL (matches the existing Rust admin
  API convention); authorization is `platform_admin?` on the authenticated
  account, mirroring `AdminNhsDmdImportPolicy`. Multipart upload streams to a
  temp file (the release is ~216 MB); body limit raised for this route only.
- **Execution** — the import runs on a spawned task instead of Solid Queue.
  The `nhs_dmd_imports` row carries the same status enum and counter columns,
  so progress is read by polling `GET .../{id}` — no Turbo/Cable dependency,
  which removes the Rails failure mode entirely.
- **Stuck-row recovery** — before insert, any active run whose `updated_at`
  is older than 30 minutes is failed with the same interruption message as
  `NhsDmdImportReconciliationJob`. This replaces the recurring job (which was
  also production-only, a second latent defect).
- **Extractor** — port `ReleaseArchiveExtractor` limits verbatim: 200
  entries, 150 MB/entry, 500 MB total, reject absolute/`..`/symlink entries.
- **Parser** — streaming `quick-xml` events replacing Nokogiri `XML::Reader`.
  Contract: `f_ampp2_3*.xml` (`AMPP` → `APPID`, `NM`, `APID`), GTIN from
  `f_gtin2_0*.xml` or nested `*GTIN.zip` (`GTINDATA` → `GTIN`, `ENDDT` under
  parent `AMPPID`), supplementary trade-family files when present.
- **Persistence** — identical semantics to `ReleaseImport`: GTIN upsert
  keyed on normalized `gtin`, `created/updated/unchanged` counting,
  `skipped_expired/missing_name/invalid` rules, delete-all + bulk insert for
  `nhs_dmd_ampp_relationships`, supplementary trade-family staging with the
  same uniqueness/reference validation.
- **Web page** — `/households/{slug}/admin/nhs-dmd-import`: upload form,
  run status with meta-refresh polling while active, history table.
- **Fixture** — build a minimal release zip in the contract test (AMPP,
  nested GTIN zip, optional supplementary files) rather than committing the
  216 MB production archive; one CI job may run the real archive manually.

## Remaining admin surface (subsequent slices)

Rails `/admin` area inventory → Rust target:

- Admin dashboard (metrics, attention queue, recent activity) — needs a
  metrics endpoint; aggregates the areas below.
- Users — index/search/pagination (`Admin::UsersIndexQuery`),
  new/create/edit/update/destroy, `activate`, `verify`,
  `membership_role` patch.
- Invitations — index/create/resend/destroy. JSON API already exists
  (`api/v1/households/{id}/admin/invitations`); needs web pages.
- Carer relationships — index/new/create/destroy/activate
  (`Admin::CarerRelationshipsIndexQuery`); no Rust API yet.
- Ambiguous person access grants — index; `person_access_grants` JSON API
  exists, needs the ambiguity projection.
- People — index (`Admin::PeopleController#index`, `PeopleIndexQuery`).
- Audit logs — index/show; JSON API exists (`admin/audit_logs`), needs
  web pages + show endpoint.
- Settings — show/update; JSON API exists (`admin/settings`), needs page.
- Household rename — edit/update (`Admin::HouseholdsController`).

Authorization parity: mirror Pundit — `platform_admin?` for dm+d import and
app settings, `household_manager?` for dashboard and most member management.
Check each policy per action; keep the port's nil-safe guard style.

## Verification

- Contract tests per area covering owner/admin/self/carer/parent/
  unauthorized where Rails policies distinguish them.
- dm+d suite: upload → status transitions → barcode/relationship counts →
  repeat-import unchanged counting → invalid-zip and concurrent-import
  rejections.
- Browser journey for the admin import page, desktop + mobile.
- Peak RSS during import must stay inside the <200 MB process budget.
