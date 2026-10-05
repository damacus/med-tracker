# Migration progress

| Tranche | State | Evidence |
| --- | --- | --- |
| 1 Foundation | Awaiting independent review | Loco fmt/check/lint/test and HTTP boot passed; Rails rollback 117 examples, CI 82 checks, cleanup 2 cases and Rails lint 1,893 files passed; desktop/mobile screenshots captured; final inventory/preservation checks passed |
| 2 Persistence/auth | Pending | schema/credential/RLS gates required |
| 3 API/integrations | Pending | shared operations and queue prerequisite |
| 4 Browser/themes | Pending | daisyUI exports and offline replay |
| 5 Workers | Pending | durable queue recovery and schedules |
| 6 Release | Pending | both-architecture scratch and rollback |

Plan review: Devin SWE-2 Max, supplied-plan only, conditionally sound. Corrections
are recorded in plan.md. Runtime checks are recorded in foundation-report.md;
independent foundation source review and implementation acceptance remain pending.

The authentication dependency decision remains open. auth-feasibility.md records
the concrete RFC7009 and client-method gaps; no bespoke protocol replacement or
external identity service has been approved as the implementation.

Independent source review dispatch was blocked by automatic approval review:
the prior Devin authorization covered plan transfer, while this invocation would
send the unpublished implementation diff and architecture records. A separate
user approval question is pending. No reviewer substitution or implementation
acceptance has occurred.

The verified foundation is published as an intermediate draft waypoint under
the repository's mandatory push instruction. Publication does not accept this
tranche or authorize bypassing the blocked Devin source-transfer approval.

Published foundation: `f54630d41e1388a5fd552d9de385ea78995b7653` on
`codex/loco-migration`, [draft PR #2451](https://github.com/damacus/med-tracker/pull/2451).
Remaining work is tracked in [#2450](https://github.com/damacus/med-tracker/issues/2450).
PRs #2397, #2399, #2402 and #2403 are closed as superseded framework choices;
their captured source and branches are preserved and useful behavior remains
part of the migration inventory. Schema, scratch, profile, notifications and
font input PRs remain open until their work is ported. PR #2381 is unchanged.
