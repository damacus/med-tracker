# Compose acceptance runner report

## Scope

The medication API acceptance task now starts a Rust API service and HTTP test
container inside its disposable Compose project. The test container shares the
API container's network namespace, so it reaches `http://127.0.0.1:39998`
inside the project; no Rust API port is published on the host. Both containers
can reach the project's `db-test:5432` database. The fixture directory is
mounted read-only into the test container. A project-specific Rust image and
Docker build layers avoid a shared mutable Cargo target between concurrent
runs. An optional `CONTRACT_TEST_SUBNET` applies a checked explicit /28 when
Docker's default address pools are exhausted.

## Evidence

- Red: `task api:contract-runner-test` failed with “Runner did not start the
  Compose API service” before implementation.
- Green: the same runner test passes its service sequence, shared network
  namespace, fixture bind, optional subnet check, relative cleanup argument,
  and injected HTTP-test failure status/cleanup checks. Fish syntax and
  `git diff --check` pass.
- Intermediate Linux smoke: Rails fixture provisioning completed in 53
  seconds; the pinned Rust 1.98.1 Bookworm image built, Rust API reached its
  health endpoint, and the real medication HTTP suites started in the test
  container. They stopped at the contract harness's non-local-origin guard.
  Sharing the API service network namespace lets the harness use loopback
  without changing its remote-target rule. An absolute fixture-directory
  environment name also shadowed the cleanup Task input; it now has a separate
  name.
- The failed smoke project's owner-checked cleanup removed its containers,
  network, volumes and image. Its retained fixture directory was removed.
- A later overlapping attempt reached a healthy Rust API in its first project,
  while Docker refused the second project's default network because its
  address pools were exhausted. Inventory found no completed contract network
  to clean. The host route table and Docker network inventory permit two
  checked /28 candidates in the former first-project range:
  `192.168.240.0/28` and `192.168.240.16/28`. Their use remains optional and
  rejects any current Docker subnet overlap. The host check compares the
  candidate network address's route with the default route; the selected
  candidates were also checked against the full host route inventory.
- `task contract:isolation` passed its fake cleanup-status, full-target
  aggregation, and restart/port/readiness checks after correcting two stale
  expectations for `smart_fhir` and `retained`. An initial real two-project
  attempt could not complete because Docker's default pool refused the second
  network. With two optional checked /28 subnets, the full gate later passed:
  `mtcontract-51c8db26d22f41a1` and `mtcontract-14daeb1198b14dd6` each
  saw its own account and PostgreSQL 18.6; first-project cleanup preserved the
  second fixture; a deliberately failed Rust-target run cleaned its own
  project without removing the second fixture. The gate exited 0. A follow-up
  inventory found no `mtcontract-*` network or owner marker for the two gate
  projects. The successful gate used escalated Docker and host-route access;
  a prior restricted run could not inspect the host route.

## Acceptance proof and limitation

The read-only runner lane's final medication acceptance pair passed 19/19
cases per project, each with exit 0, using distinct /28 subnets and fixtures.
Its exact project IDs, input digest, and evidence limits are in
[mobile-oauth-runner-report.md](mobile-oauth-runner-report.md). The original
43-file pre/post manifest matched, but omitted the new subnet override and
validator files; that manifest does not prove their byte-for-byte stability.
The successful broader isolation gate above provides separate fixture and
cleanup evidence. Default Docker network allocation remains exhausted on
this host; concurrent runs require checked explicit subnets until that host
capacity changes.
