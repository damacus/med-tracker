# Proposal

## Why

Rust browser users cannot download the health-history PDF already produced by the report API. Provide a small Reports page so permitted users can select a person and date range and obtain the existing report.

Originating issue: https://github.com/damacus/med-tracker/issues/2387

## What Changes

- Add a Reports destination with person, start date, end date and include medication takes controls.
- Download the existing PDF with preserved attachment and privacy headers.
- Show accessible filter errors and download failures, with localised desktop and mobile UI.
- Non-goals: report previews, medication-review reports, new report generation, exports, charts, email, background jobs, Rails changes, migrations and new access policies.

## Capabilities

### New Capabilities

- `browser-health-history-reports`: select and download authorised health-history PDFs through the Rust browser.

### Modified Capabilities

None.

## Impact

Rust browser routes and rendering, a bounded binary response path in the existing authenticated internal API adapter, navigation, locale catalogues and browser tests. The existing report API retains authorisation, generation and audit ownership. No PDF or crypto dependencies are added.
