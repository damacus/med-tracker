# Issue #1997: HTML-based PDF reports

Spec: https://github.com/damacus/med-tracker/issues/1997

## Context

Deliver two stacked pull requests. The lower branch establishes and proves the
in-process renderer foundation. The upper branch migrates both exports and
polishes their download controls. The local compatibility spike already proved
the published `sghtmltopdf 0.1.1` native gem under Ruby `4.0.6` on
`aarch64-linux` and `x86_64-linux`; both returned `%PDF-1.7` when given a TTF.

## Global Constraints

- Work in the existing isolated worktree. The lower branch is
  `codex/1997-pdf-renderer-foundation` from refreshed `origin/main`; the upper
  branch is `codex/1997-html-pdf-reports` from the lower branch.
- Follow Red-Green-Refactor. Run `task test:preflight` before implementation,
  focused `task test TEST_FILE=...` checks during each task, then repository
  quality gates at each pull-request boundary.
- Use Ruby `4.0.6`, Rails `8.1.3`, PostgreSQL `18`, Phlex, RubyUI, Stimulus,
  RSpec fixtures, and repository `task` commands. Add no ERB files.
- Never add or remove code comments. Preserve unrelated changes.
- Keep rendering in-process. Add no renderer service, database migration,
  background job, or browser-based PDF renderer.
- Preserve the existing routes, filenames, filter parameters and semantics,
  authorization boundaries, query/data separation, attachment responses,
  `application/pdf`, `Cache-Control: no-store`, and public constructors plus
  `#render` for both report services.
- Keep #1996 clinical content out of scope: no new conditions, chronology,
  measurements, linked as-needed doses, recurrence, portable export, or
  date-range behavior.
- Bundle a redistributable SIL Open Font Licence Noto Sans TTF with its licence.
  Cover English, Welsh, Spanish, Irish, and Portuguese deterministically.
- Restrict local reads to the bundled font directory and keep remote asset
  fetching disabled. Do not log report HTML, names, notes, evidence text, or
  other health data when rendering fails.
- Treat PDF as a print/share format. Do not claim tagged PDF, PDF/UA, or full
  assistive-technology support; the existing HTML screens remain the accessible
  alternative.
- Before every commit, verify author, committer, and signing identity as
  `Dan Webb <dan.webb@damacus.io>`. Use Conventional Commits. Before every
  push run `task rubocop`, `task test`, and the other task-specific gates.

## Task 1: Add and prove the native renderer dependency and font

On `codex/1997-pdf-renderer-foundation`, add and lock exactly
`sghtmltopdf 0.1.1` while retaining `prawn` and `prawn-table`. Bundle a static
Noto Sans TTF that covers all five locales plus the upstream SIL Open Font
Licence. Add the smallest failing contract spec first, then prove the gem loads
and renders `%PDF-1.7` under the application test image with the bundled font.

Add a compatibility record covering the already observed ARM64 and emulated
AMD64 Ruby `4.0.6` native-gem names, the WOFF2 rejection, successful TTF render,
the application image build result for both `linux/amd64` and `linux/arm64`,
build/image-size impact, representative warm render time and peak memory, and
why server mode is not selected. Document the font licence and the renderer's
unsupported CSS, tagged-PDF, PDF/UA, and assistive-technology limitations.

Acceptance: dependency resolution needs no Rust or libclang in MedTracker's
image; both target architectures build and render a valid PDF in-process; no
existing endpoint changes yet; `prawn` remains available.

## Task 2: Build the shared Phlex PDF renderer and A4 design system

Add `Reports::PdfRenderer` with
`#render(component:, metadata:)` and `Reports::PdfRenderer::Error`. It must
render a full-document Phlex component synchronously to bytes, pass title,
author, subject and related metadata to sghtmltopdf, and normalize only known
renderer failures into the application error while preserving the cause.

Add shared full-document Phlex layout components and an inline dedicated print
stylesheet derived from the existing medication-review palette. Provide A4
margins, MedTracker header, report title/context, generated timestamp, page
counters, restrained print-safe colors, section headings, callouts, empty
states, tables with repeating headers, `break-inside`, widows/orphans and
heading-orphan controls. Configure the explicit bundled font. Permit only the
font directory for local access and prove arbitrary local and remote asset
loads fail closed.

Write failing component and renderer specs first. Cover valid PDF bytes,
metadata/text extraction, all locale glyphs, empty/short/long/multi-page HTML,
pagination CSS, asset restrictions, and error normalization. Do not query the
database from components.

Acceptance: shared renderer and layout are independently usable and tested;
existing endpoints still use Prawn. Run the lower-branch full review and all
PR 1 quality gates after this task.

## Task 3: Migrate the health-history export to Phlex

On dependent branch `codex/1997-html-pdf-reports`, replace the Prawn internals
of `Reports::HealthHistoryPdf` with dedicated report Phlex components using the
shared renderer. Preserve the constructor and `#render` interface, all current
sections, translated text, person and date context, generated timestamp,
metadata, event and medication rows, empty states, pattern summaries and
disclaimer. Do not move or duplicate query logic into the components.

Write failing component/service specs before production changes. Cover empty
data, long names, long notes, locale glyphs, a large table and a multi-page
report. Extend the request spec only where needed to preserve existing route,
authorization, filters, filename, attachment, media type and no-store behavior.

Acceptance: health history exclusively uses sghtmltopdf, remains content- and
HTTP-compatible, and has no Prawn reference.

## Task 4: Migrate the medication-review export and translations

Replace the Prawn internals of `Reports::MedicationReviewPdf` with dedicated
report Phlex components using the same shared layout. Preserve the constructor
and `#render`, existing evidence content, match explanations, source links,
risk language/colors, summary counts, practitioner outcomes, grouping,
metadata, boundary statement, empty state and default visible export scope.

Move every remaining hard-coded report string into structurally identical
`en`, `cy`, `es`, `ga` and `pt` locale nodes with unchanged interpolation
variables. Write failing service/component specs first and update request
coverage to parse rendered PDF text and metadata rather than scan raw object
bytes.

Acceptance: medication review exclusively uses sghtmltopdf, no report copy is
hard-coded, every locale tree is synchronized, and current HTTP/filter behavior
is preserved.

## Task 5: Add safe failure handling and accessible export controls

Render each complete PDF before `send_data`. Rescue only
`Reports::PdfRenderer::Error`, log the report type and exception class without
health data, and redirect to the originating HTML screen with a translated
generic alert. Prove failures never return `application/pdf` or partial bytes.

Replace the bare links with one reusable RubyUI/Phlex export panel. Health
history must show the active people and date range and preserve its current URL
parameters. Medication review must clearly state that the export contains the
existing default visible review scope and must keep its current URL semantics.

Add a Stimulus controller that, after keyboard or pointer activation, exposes
`aria-busy=true` and `aria-disabled=true`, changes the visible label to the
translated equivalent of `Preparing PDF...`, prevents duplicate activation,
then safely resets without moving or trapping focus. Add equivalent locale
nodes in all five files.

Write failing request, component and browser specs first. Verify keyboard use,
busy/reset behavior, mobile layout, scope copy and safe failures.

Acceptance: both controls are responsive and accessible, current downloads
remain compatible, and renderer failures are safe and translated.

## Task 6: Remove Prawn and complete visual and stack verification

After Tasks 3-5 are green, remove `prawn`, `prawn-table`, obsolete Prawn modules
and lockfile dependencies. Keep only the new renderer path. Generate
deterministic short, empty, long and multi-page PDFs into `tmp/pdfs/`, render
every page to PNG with Poppler, and inspect A4 output for clipping, overlaps,
row splits, repeated headers, orphaned headings, missing glyphs and page
furniture. Fix every visual defect found before continuing.

Capture desktop and mobile screenshots for both export controls under
`docs/screenshots/issues/1997/`. Run focused specs, locale-tree validation,
`task rubocop`, `task test`, `task brakeman`, relevant browser tests,
`git diff --check`, and the appropriate production multi-architecture build.

Acceptance: Prawn is absent, all automated and visual checks pass, both PR
layers have independent reviewable diffs, and the complete stack has a clean
Sol `medium` whole-stack review before publication.
