# Tasks

The change is delivered as a stack of small application PRs against `main`,
plus one home-ops PR. Each PR follows Red-Green-Refactor with focused
`rtk task test:integration` runs before `rtk task ci`.

## 1. Spec (docs only)

- [ ] 1.1 Author this OpenSpec change (proposal, design, tasks, delta specs)
      and validate with `openspec validate add-rust-otel-observability --strict`.
- [ ] 1.2 Add ADR 0020 recording `trace_id` in API error bodies and the W3C
      trace ID in `x-medtracker-trace-id`; run `task docs:build` and
      `git diff --check`.

## 2. Core telemetry (application PR 2)

- [ ] 2.1 RED: failing tests for the exact JSON log shape (no ANSI), span-field
      merge, generic key rule drops and `dropped_keys`, 256-byte value cap,
      disabled-by-default (no provider, single startup line), inbound
      `traceparent` sampled=0 join-and-record, ratio parsing, and fail-open
      export with one rate-limited warning then a recovery line via a
      controllable fake exporter (no timing assertions).
- [ ] 2.2 GREEN: `src/telemetry/` module (config, init via
      `Hooks::init_logger`, JSON log layer, key rule, root ratio sampler,
      exporting wrapper, self-diagnostics rate limiter, 2 s shutdown flush)
      plus `settings.telemetry` in `config/*.yaml` via `get_env`.
- [ ] 2.3 GREEN: request server span replacing `request_trace`, W3C
      `x-medtracker-trace-id`, sentinel privacy tests over logs and the
      in-memory exporter, and conversion of existing `tracing::` call sites
      to conforming keys.
- [ ] 2.4 GREEN: build identity (`MEDTRACKER_VERSION`/`MEDTRACKER_COMMIT`
      through Dockerfile, `loco-images.yml`, `build.rs`, `app_version()`).
- [ ] 2.5 Record `task telemetry:bench` results (export off and local sink)
      and nextest wall-time comparison in the PR description.

## 3. Units of work (application PR 3)

- [ ] 3.1 Worker `job {name}` spans, scheduler `task {name}` root spans, panic
      hook, DB accounting layer and `set_metric_callback` wiring, outbound
      HTTP client spans; bench evidence recorded.

## 4. Imports (application PR 4)

- [ ] 4.1 Typed `ImportFailure` categories, nullable `trace_id` migration on
      `nhs_dmd_imports`, failed-import page reference, portable-import stage
      events and reconcile-task span; browser screenshots of the reference.

## 5. Metrics (application PR 5)

- [ ] 5.1 OTLP periodic reader and the instruments listed in the design,
      including `pg_loco_queue` gauges and the documented SQLx pool-stat
      exception.

## 6. API trace_id (application PR 6)

- [ ] 6.1 `trace_id` in API error bodies, OpenAPI update, regenerated
      clients.

## 7. Domain events (application PR 7)

- [ ] 7.1 Dose, reminder/push, mail/invitation and auth/rate-limit/support
      events with `medtracker.outcome`, committed-only-after-commit
      semantics.

## 8. Docs and archive (application PR 8)

- [ ] 8.1 Rewrite `docs/operations/application-observability.md` for Rust,
      remove the Rails canary procedure, archive this change after all PRs
      merge.

## 9. home-ops PR (separate repository)

- [ ] 9.1 On top of the Tempo-retirement branch: single-node VictoriaTraces
      (pinned chart, PVC, 7-day retention, resources, health checks, scrape,
      network policy allowing only medtracker-canary pods), Grafana
      Tempo-type data source and Loki derived `trace_id` field, VictoriaMetrics
      network policy, medtracker-canary OTLP settings, alert rules and canary
      dashboard update. Production `home` untouched; no live cluster actions.
