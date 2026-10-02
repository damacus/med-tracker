# Route verification step

This is a bounded verification step within the approved sweep, after the four interaction fixes
and their review corrections. Nightingale remains the sole product/test writer; Hubble reviews.

## Ownership

Own `spec/system/mobile_ui_audit_spec.rb` and the matching verification report only. Reuse existing
helpers and fixtures. Do not make production fixes in this step; report a reproducible failure to
the coordinator before assigning a narrowly scoped correction.

## Required changes

- Expand authenticated and signed-out route checks to light/dark at 390/1280 pixels.
- Add stock check, medicine reviews, health history, household settings, password-reset request
  and verification-resend routes to the existing route lists where available.
- Assert the intended destination, rather than counting a login or unauthorised redirect as a
  successfully audited page. Use the canonical household dashboard instead of the root redirect.
- Use a valid invitation for invitation-only account creation. Do not fake unavailable stateful
  authentication links. Record unsupported recovery, verification and passkey states explicitly.
- Keep ordinary household-owner coverage separate from the platform-only import page. Use an
  explicitly authorised platform fixture for that page if the existing helper permits it, otherwise
  classify that page as uncovered rather than treating its denial as a pass.
- Preserve the existing admin and carer journey checks. Combine their results with the separately
  recorded parent/self/nurse fixture browser evidence without claiming a role from an account name.

Run only this file with the isolated `task test TEST_FILE=...` command. Record exact examples,
failures, route/state coverage and limitations in `04-route-verification-report.md`. Send review-ready
and return after this step. Long-content candidates and final screenshot capture are subsequent
small assignments; the coordinator owns the full suite, lint, documentation gates and publication.
