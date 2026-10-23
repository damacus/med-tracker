# Loco scheduler patch

MedTracker retains the published `loco-rs` 1.2.0 release and carries one local
scheduler correction. The release's native-task path concatenates an executable
and task arguments into a command string, then always invokes `/bin/sh -c` on
Unix. The production scratch image deliberately contains no shell.

The public Loco hooks do not replace the scheduler executor. Both the CLI
`scheduler` command and `start --scheduler` construct the framework scheduler
directly. The latest published release was still 1.2.0 on 23 October 2026, and
the upstream scheduler retained this behaviour. Replacing the application CLI or
scheduling loop would duplicate more framework behaviour than this patch.

`upstream/loco-rs-1.2.0.crate` is the original crates.io release archive. Its
SHA-256, retained from the registry lockfile, is:

```text
5386baf405736a2620001f4561b00fa4874dae99eac3ccf01713fabc9ef13af4
```

`loco-rs-1.2.0/` contains that complete archive, with only the changes in
`loco-scheduler.patch`. `task scheduler:patch:check` checks the archive digest,
complete file inventory and exact reviewed diff, and tests rejection of
tampering. The published version, licence and original sources remain available
for an offline comparison. No Git dependency or unreleased framework code is used.

The correction keeps the existing YAML `run` string. Native jobs parse its
quoting with the released `shlex` parser, then pass a separate executable and
argument vector to the existing `duct` process library. Quotes group arguments;
variables, command substitutions, globs, pipelines and redirects do not expand.
Empty task names, NUL bytes and malformed quoting fail before execution.
Explicit `shell: true` jobs retain their platform-shell path and require that
shell to be available. The scratch production image cannot run shell jobs.

Cron parsing, tags, job filters, startup jobs, blocking-task isolation and
scheduler shutdown remain owned by Loco. Native children retain inherited
environment variables and receive the scheduler's `LOCO_ENV`. Existing output
modes and failure reporting remain part of the process acceptance checks.

Remove the patch when a published framework release supports direct native-task
execution. Re-run the process and scratch-image scheduler acceptance tests before
switching back to the registry dependency; do not remove the tests with the patch.

Sources: [Loco releases](https://github.com/loco-rs/loco/releases),
[published crate](https://docs.rs/crate/loco-rs/1.2.0).
