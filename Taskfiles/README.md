# Task runner guide

MedTracker uses [Task](https://taskfile.dev/) as the public entry point for
development, tests, documentation, security checks, and local production-image
validation. Run `task -l` for the current command list.

Do not call Docker Compose or Rails commands directly when a Task command owns
the workflow.

## File layout

`Taskfile.yml` defines repository-wide commands and includes the task files in
this directory.

| File | Scope |
| --- | --- |
| `internal.yml` | Shared Docker Compose operations |
| `dev.yml` | Development services and data |
| `test.yml` | Docker-backed test services |
| `local.yml` | Host test commands with a local database |
| `prod.yml` | Local production-image checks and operator wrappers |
| `docs.yml` | Documentation build and preview |
| `audit.yml` | Audit export and verification |
| `lighthouse.yml` | Browser quality checks |
| `openspec.yml` | OpenSpec validation and status |
| `worktree.yml` | Worktree creation and cleanup |

Keep user-facing commands in the file that owns their environment. Put shared
Compose mechanics in `internal.yml`.

## Common commands

```fish
task test
task test TEST_FILE=spec/models/person_spec.rb
task test:preflight
task rubocop
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

## Compose isolation

The shared internal tasks use one `compose.yaml` with `dev`, `test`, and `prod`
profiles. The Compose project name contains the worktree directory name and a
hash of its absolute path. This keeps containers, networks, and volumes separate
between worktrees.

Every internal Compose task requires an `ENVIRONMENT` value. Public task files
pass that value when they call `internal:*` tasks.

`internal:run` starts a temporary service container through the worktree's
Compose lock, so pass the complete command once. The root Taskfile uses
`run: once`, which can deduplicate repeated calls to the same internal task.

## Add a public command

1. Choose the task file that owns the command's environment or purpose.
2. Reuse an `internal:*` task for shared Compose work.
3. Pass required values through `vars`, `env`, or both.
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
rg 'internal:task-name|task: task-name' Taskfile.yml Taskfiles
```

Verify the narrow public commands that use the changed boundary. Run the normal
repository gates when executable task configuration changes.

## Variables

Common public variables include:

| Variable | Use |
| --- | --- |
| `TEST_FILE` | Limit `task test` to a spec path |
| `AUTOCORRECT` | Enable RuboCop correction |
| `NO_CACHE` | Rebuild a selected image without Docker cache |
| `COMPONENTS` | Select RubyUI families for comparison |
| `OUTPUT` | Set an external RubyUI comparison directory |

Each operator command can define additional required variables. Read its
`summary` with `task --summary <task-name>` before use.

## Troubleshooting

- If Task cannot find a command, run `task -l` and use its full namespace.
- If an internal task reports a missing variable, check the caller's `vars` and
  `env` blocks.
- If Docker uses an unexpected service or volume, confirm the selected profile
  and worktree before changing data.
- If one wrapper needs several different container commands, define separate
  public tasks instead of repeatedly calling `internal:run` during the same
  execution.
