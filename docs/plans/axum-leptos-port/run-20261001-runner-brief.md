# Runner brief

Own deterministic checks, temporary logs and generated screenshots only. No source edits,
commits or publication. Coordinator owns task wrapper changes.

Use Fish, rtk and task. Keep one heavy build active initially. Use disposable synthetic
fixtures and an isolated Compose project. Existing full runner cleans up its own project;
a later persistent harness must retain its ownership marker and use the same guarded cleanup.

Network diagnostics require Docker socket access. Validation accepted `10.243.18.0/28`
after checking existing Docker networks and host routes. The validator requires `/28`.

First RED command:

```fish
set -gx CONTRACT_TEST_SUBNET 10.243.18.0/28
rtk task api:browser-rust BROWSER_TEST_FILES=tests/household-routes.test.mjs
```

Log: `/tmp/medtracker-runner-route-red.log`. Source snapshot occurs after the
dashboard build dependency completes; notify coordinator at the printed source digest.

Deadline: 2026-10-01 10:27 UTC. Stop adding journeys at 10:12 UTC.
