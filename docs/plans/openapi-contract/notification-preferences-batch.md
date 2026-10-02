# Notification preference batch

User direction: complete a low-risk API slice within the next five percentage
points of allowance. Starting usage was 94%; reserve the remainder for review
and publication. No reset credit is authorised.

Scope: `getNotificationPreference`, `updateNotificationPreference` and
`replaceNotificationPreference`. These are three absent operations in the
unchanged original 89-operation baseline. Location operations are outside that
baseline and must not increase its completed count.

## Acceptance

- GET returns the signed-in person's preference, exact documented fields,
  formatted nullable times and an ETag; absence returns 404.
- PATCH and PUT merge supplied attributes, create defaults when absent, and
  preserve omitted values. Strict booleans, times, wrappers and attribute
  names follow the documented schema. Invalid requests do not change state.
- Current household and person view/manage permissions apply. A caller cannot
  select another person's preference. Authentication and rate-limit wiring
  reuse proven shared behaviour with direct endpoint assertions.
- Preserve source audit and sync effects, stable no-op behaviour and atomic
  writes. No notification sending, external service or schema migration.

## Ownership

Sol owns production, specification clarifications and runner wiring. Luna
owns the new notification preference HTTP test file. Tests demonstrate RED
before production changes, then the two lanes proceed independently. The
orchestrator owns independent review, the evidence ledger and publication.
Use existing isolated Compose projects, project Task commands and SeaORM.
Do not expand into another API family without checking remaining allowance.

## Completion evidence

Initial RED: PATCH returned 404 instead of 200 in project
`mtcontract-3988f21dccba4cf5`, log `1790411964_task_api_624fdf.log`.
Final isolated GREEN: both HTTP groups passed in
`mtcontract-26b8a402e0c5404a`, log `1790412885_task_api_624fdf.log` under
`/Users/damacus/Library/Application Support/rtk/tee/`. Cleanup completed.
The groups cover lifecycle/validation/permissions and three-route rate limiting.
An intermediate run exposed an incorrect test identity; the final guard uses
the real person's membership and restores its exact role and grant state.
The corrected test target compiled before the final acceptance build.

Independent requirements and code review passed after correcting GET denial
to 404, rejecting leap seconds, and moving the write permission check under
the household lock. API formatting, Clippy, 15 unit tests, selected contract
compilation and runner dispatch/failure-cleanup checks passed. Shared helpers
supply authentication, audit and limiter semantics; the evidence ledger lists
the endpoint assertions rather than claiming every permutation was tested.

At 95% usage, the next two native-device-token operations were approved within
the same budget. Their scope is in `native-device-tokens-batch.md`.
