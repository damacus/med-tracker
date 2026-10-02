# Task runner guide

MedTracker uses [Task](https://taskfile.dev/) as the public entry point for
development, tests, documentation, security checks, and local production-image
validation. Run `task -l` for the current command list.

Do not call Docker Compose or Rails commands directly when a Task command owns
the workflow.

## File layout

`Taskfile.yml` includes the files owned by each application and tool. Application
commands live beside their owner; this directory holds repository automation.
Existing root commands remain available while callers adopt the explicit namespaces.

| File             | Scope                                               |
|------------------|-----------------------------------------------------|
| `../rails/Taskfile.yml` | Rails commands and environment includes       |
| `../rails/tasks/internal.yml` | Shared Docker Compose operations         |
| `../rails/tasks/dev.yml` | Development services and data                  |
| `../rails/tasks/test.yml` | Docker-backed test services                   |
| `../rails/tasks/local.yml` | Host tests with a local database             |
| `../rails/tasks/prod.yml` | Local production-image checks                 |
| `../rails/tasks/audit.yml` | Audit export and verification                |
| `../rails/tasks/lighthouse.yml` | Browser quality checks                  |
| `../rust/Taskfile.yml` | Rust API, web and UI package commands            |
| `../rust/contract-tests/Taskfile.yml` | Shared API compatibility and client generation |
| `../client-tools/Taskfile.yml` | CLI and MCP Cargo workspace               |
| `../client-tools/openapi-generator/Taskfile.yml` | Native API generation  |
| `../docs/Taskfile.yml` | Documentation build and preview                  |
| `../mobile/android/Taskfile.yml` | Android build and checks               |
| `ci.yml`        | CI classification, checks and gate                   |
| `agents.yml`    | Translator integration                               |
| `openspec.yml`   | OpenSpec validation and status                      |
| `worktree.yml`   | Worktree creation and cleanup                       |

Keep user-facing commands in the file that owns their environment. Put shared
Compose mechanics in `rails/tasks/internal.yml`.

Rails source and Docker configuration still live at the repository root. Only
its Taskfiles have moved under `rails/`; the source relocation is a separate change.
`Taskfiles/local.yml` is a compatibility symlink for the existing database-login spec.

## Namespaces and working directories

Prefer `rails:*`, `rust:api:*`, `rust:web:*`, `rust:ui:*`, `contracts:*`,
`contracts:clients:*`, `client-tools:*`, `mobile:android:*` and `docs:*`.
The old Rails commands, `api:*`, `contract:*`, `api-clients:*` and `android:*`
remain available. New code should use the explicit namespace.

Root and local invocation use the same commands and working directories:

```fish
task rails:rubocop
task -d rails rubocop
task rust:api:check
task -d rust/api check
task client-tools:check
task -d client-tools check
task mobile:android:test
task -d mobile/android test
task docs:build
task -d docs build
```

Rails, API compatibility runners and API generation execute from the repository
root because they use root-owned Compose, scripts, contracts and output paths.
The client-tools, Android, web and UI packages execute from their own directories.
Explicit task directories also make direct local invocation work.

`task rust:build` compiles the UI and API packages. It does not rebuild a running
container. Existing `api:contract-up` and `rust:api:contract-up` commands build
and start only a named disposable contract environment; they are not development
or production restart commands. This refactor does not introduce a Rust development
runtime or change database-reset behaviour.

## Common commands

```fish
task test
task test TEST_FILE=spec/models/person_spec.rb
task test:preflight
task rubocop
task rubocop:test
task brakeman
```

```fish
task dev:up
task dev:seed
task dev:logs
task dev:stop
```

```fish
task docs:build
task docs:serve
task openspec:validate
```

`task dev:rebuild`, `task test:rebuild`, and `task prod:rebuild` delete the
selected environment's database volumes. Use them only when a clean database is
required.

RuboCop runs with the Performance plugin and preview behaviour for the next
major release. The project explicitly keeps its quote and indentation styles,
selected complexity limits, and file exclusions. Convention offences still fail
CI. Use `task rubocop:test` to check that performance findings and style failures
remain enforced when RuboCop or its configuration changes.

## Compose isolation

The shared internal tasks use one `compose.yaml` with `dev`, `test`, and `prod`
profiles. The Compose project name contains the worktree directory name and a
hash of its absolute path. This keeps containers, networks, and volumes separate
between worktrees.

Every internal Compose task requires an `ENVIRONMENT` value. Public task files
pass that value when they call `internal:*` tasks.

`internal:run` starts a temporary service container through the worktree's
Compose lock, so pass the complete command once. The root Taskfile defaults to
`run: once`, which would deduplicate repeated calls to the same task, so every
internal helper declares `run: always`. These helpers are imperative operations
where deduplication would silently skip requested work.

## Add a public command

1. Choose the task file that owns the command's environment or purpose.
2. Reuse an `internal:*` task for shared Compose work.
3. Pass required values through `vars` — task-level `env` does not cross `task:` calls, so an `internal:*` callee can only forward into its container env values it received through `vars`.
4. Add a short `desc` so the command appears clearly in `task -l`.
5. Add a `summary` when the command needs usage examples or safety notes.

Example:

```yaml
console:
  desc: Open the development Rails console
  cmds:
    - task: internal:run
      vars:
        ENVIRONMENT: dev
        COMMAND: rails console
```

Use `requires.vars` when a command must refuse missing input. Use environment
transport for values that can contain spaces or shell metacharacters. Do not
interpolate untrusted values into a shell command.

## Change an internal task

An internal task can affect development, test, and local production-image
workflows. Check every caller before changing it:

```fish
rg 'internal:task-name|task: task-name' Taskfile.yml Taskfiles rails rust
```

Verify the narrow public commands that use the changed boundary. Run the normal
repository gates when executable task configuration changes.

## Variables

Common public variables include:

| Variable      | Use                                           |
|---------------|-----------------------------------------------|
| `TEST_FILE`   | Limit `task test` to a spec path              |
| `AUTOCORRECT` | Enable RuboCop correction                     |
| `NO_CACHE`    | Rebuild a selected image without Docker cache |
| `COMPONENTS`  | Select RubyUI families for comparison         |
| `OUTPUT`      | Set an external RubyUI comparison directory   |

Each operator command can define additional required variables. Read its
`summary` with `task --summary <task-name>` before use.

## Troubleshooting

- If Task cannot find a command, run `task -l` and use its full namespace.
- If an internal task reports a missing variable, check the caller's `vars` and
  `env` blocks.
- If Docker uses an unexpected service or volume, confirm the selected profile
  and worktree before changing data.
- A wrapper can call `internal:run` several times in one execution: internal
  helpers declare `run: always`, so each call runs in order instead of being
  deduplicated.
