# Tasks

PR #3037 delivers application slices 1–4 together against `main`.
Slices 5–9 remain follow-up work; this checklist covers the full change rather
than only this PR. Checked items record implemented behaviour and verification,
not a claim that every original commit's RED stage was observed.

Owner decision, 2026-11-04: performance evidence is excluded from this PR's
completion criteria. No latency or test-runtime improvement is claimed.
Task 2.5 remains unchecked as explicitly deferred work; the benchmark script
also needs process isolation, endpoint/port handling and failed-sample accounting
before its output can be used as evidence.

Verification for slices 1–4 includes telemetry unit tests, restricted-runtime
integration tests, desktop/mobile import-reference browser journeys, strict
OpenSpec validation and documentation build. Required application verification
uses `rtk task ci` with the normal profiles, including eight API workers.
The run-owned API fixture connections reuse generic query plans, with live
authority checked on every request. The earlier worker-budget experiment did
not resolve the intermittent request timeouts and was discarded. Foundation
commit `fa20db7c0` passed hosted CI and both Linux image acceptance workflows.
Checklist completion does not mean merged or deployed.

Stage 5.1 is a separate metrics follow-up dependent on PR #3037. It targets
the verified foundation in the owner-requested native stack #3040:
PR #3037 → PR #3039 → the stage 6.1 trace-ID follow-up. Each upper PR targets
the branch below it, with `main` as the stack trunk. Stages 7–9 remain outside
the trace-ID follow-up. Stack membership does not authorise merging or deployment.

## 1. Spec (docs only)

- [x] 1.1 Author this OpenSpec change (proposal, design, tasks, delta specs)
      and validate with `openspec validate add-rust-otel-observability --strict`.
- [x] 1.2 Add ADR 0020 recording `trace_id` in API error bodies and the W3C
      trace ID in `x-medtracker-trace-id`; run `task docs:build` and
      `git diff --check`.

## 2. Core telemetry (application PR 2)

- [x] 2.1 Regression coverage for the exact JSON log shape (no ANSI), span-field
      merge, generic key rule drops and `dropped_keys`, 256-byte value cap,
      disabled-by-default (no provider, single startup line), inbound
      `traceparent` sampled=0 join-and-record, ratio parsing, and fail-open
      export with one rate-limited warning then a recovery line via a
      controllable fake exporter (no timing assertions).
- [x] 2.2 GREEN: `src/telemetry/` module (config, init via
      `Hooks::init_logger`, JSON log layer, key rule, root ratio sampler,
      exporting wrapper, self-diagnostics rate limiter, 2 s shutdown flush)
      plus `settings.telemetry` in `config/*.yaml` via `get_env`. Deadline
      regressions cover process exit after stalled SDK flush/shutdown, overlap,
      errors and processor panic recovery.
- [x] 2.3 GREEN: request server span replacing `request_trace`, W3C
      `x-medtracker-trace-id`, sentinel privacy tests over logs and the
      in-memory exporter, and conversion of existing `tracing::` call sites
      to conforming keys.
- [x] 2.4 GREEN: build identity (`MEDTRACKER_VERSION`/`MEDTRACKER_COMMIT`
      through Dockerfile, `loco-images.yml`, `build.rs`, `app_version()`).
- [ ] 2.5 DEFERRED by owner on 2026-11-04: record `task telemetry:bench`
      results (export off and local sink)
      and nextest wall-time comparison in the PR description.

## 3. Units of work (application PR 3)

- [x] 3.1 Worker `job {name}` spans, scheduler `task {name}` root spans, panic
      hook, DB accounting layer and `set_metric_callback` wiring, outbound
      HTTP client spans. Performance evidence is excluded by the owner decision
      above.

## 4. Imports (application PR 4)

- [x] 4.1 Typed `ImportFailure` categories, nullable `trace_id` migration on
      `nhs_dmd_imports`, failed-import page reference, portable-import stage
      events and reconcile-task span; browser screenshots of the reference.

## 5. Metrics (follow-up application PR 5)

- [x] 5.1 OTLP periodic reader and the instruments listed in the design,
      including `pg_loco_queue` gauges and the documented SQLx pool-stat
      exception. Verified by decoded OTLP request/queue/pool acceptance,
      disabled and unavailable exporter checks, independent signal recovery,
      bounded lifecycle tests and full `rtk task ci` with normal profiles.

## 6. API trace_id (follow-up application PR 6)

- [x] 6.1 `trace_id` in API error bodies, OpenAPI update, regenerated
      clients. Implementation and independent review are complete locally.
      Focused server tests, Rust client/CLI checks, Android error-adapter tests,
      generator determinism and Swift/Kotlin builds passed. The final unchanged
      candidate passed full `task ci` with normal profiles: 273 unit tests,
      two canary tests, 1,189 API tests, template concurrency and 68 browser
      journeys. The trace-ID follow-up is published as PR #3042, and hosted
      CI passed on commit `136585970`. Earlier runs reproduced sharing
      schedule-setup timeouts; PR #3030 replaced prerequisite setup with
      fixtures and merged after its hosted checks passed. A separate test
      crossed UTC midnight; its same-day safety assertion was corrected in
      PR #3042. No deadline, worker-profile, RLS or planner changes were
      retained, and no overall latency improvement is claimed.

## 7. Domain events (follow-up application PR 7)

- [x] 7.1a API medication-take attempts, committed creates and replays emit
      named, privacy-safe events. Committed and replayed events are emitted
      only after the outer request transaction commits; a request-audit failure
      rolls back the take without a committed event. Focused restricted-runtime
      API and tracing-to-exporter checks cover the behaviour.
- [ ] 7.1 Dose, reminder/push, mail/invitation and auth/rate-limit/support
      events with `medtracker.outcome`, committed-only-after-commit
      semantics. Dose paths outside API medication-take, blocked/rolled-back
      events and all other event families remain outstanding.

## 8. Docs and archive (follow-up application PR 8)

- [ ] 8.1 Rewrite `docs/operations/application-observability.md` for Rust,
      remove the Rails canary procedure, archive this change after all PRs
      merge.

## 9. home-ops follow-up PR (separate repository)

- [ ] 9.1 On top of the Tempo-retirement branch: single-node VictoriaTraces
      (pinned chart, PVC, 7-day retention, resources, health checks, scrape,
      network policy allowing only medtracker-canary pods), Grafana
      Tempo-type data source and Loki derived `trace_id` field, VictoriaMetrics
      network policy, medtracker-canary OTLP settings, alert rules and canary
      dashboard update. Production `home` untouched; no live cluster actions.
