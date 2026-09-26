# Location reads review

Reviewed against `docs/api/openapi.v1.yaml` on 25 September 2026.

## Scope

The first slice adds `listLocations` and `getLocation`, using SeaORM and the
existing authentication and audit boundaries. It does not establish complete
coverage of either operation or of the 118-operation API.

## Requirements review

The resource representation contains the five documented fields and no extras.
Collection and detail envelopes match their documented shapes. Both queries
filter by the routed household. Detail lookup supports numeric and portable
identifiers. The detail response reuses the existing ETag implementation.

The first acceptance run demonstrated missing routes, then passed after their
implementation. Review identified silently clamped pagination values in the
shared parser. A subsequent test demonstrated `page=0` returning 200 instead
of 422. Validation is now specific to locations, preserving the scope of this
slice. The final isolated acceptance run passed all three tests, including
all six invalid-filter cases and numeric and portable detail identifiers.
Requirements review passes for this explicitly partial slice.

## Code review

Code review passes with no blocking issue identified. Database values
use SeaORM filters, and ordinary reads introduce no raw SQL. The existing
transaction boundary commits audit records with the response. No Leptos or
Rails application files were changed.

The shared authentication, audit, and ETag implementations were reused; this
review does not certify their complete behaviour across all documented error
cases. The operation evidence report must retain those untested requirements.

## Checks performed by the reviewer

- `task api:contract-runner-test`: passed for existing runner sequences and
  failure cleanup. It does not explicitly exercise the new dispatch branch.
- `task docs:build`: passed after allowing access to the dependency cache and
  package network. Initial sandbox failures were environmental.
- `git diff --check`: passed at source review.
- Final acceptance log inspected: `1790362281_task_api_7d11fd.log`, three
  passing tests, zero failures, and owned storage cleanup.

## Follow-up

The Docker runner recompiles the existing medication acceptance suite even
when selecting a small OpenAPI test target. Measure and narrow this build
work before scaling the same loop across the remaining operations.
