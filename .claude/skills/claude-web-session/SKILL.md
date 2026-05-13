---
name: claude-web-session
description: Manage Claude Code web sessions, remote handoff, and teleport workflows for MedTracker work.
allowed-tools: Bash(claude *), Bash(service postgresql start), Bash(service redis-server start), Bash(docker compose *), Bash(task *), Bash(git status*), Bash(git pull*), Bash(git push*)
---

# Claude Web Session Workflow

Use this playbook when working with `claude --remote` and `--teleport`.

## Start a session

```bash
claude --remote "task test TEST_FILE=spec/path/to/file_spec.rb"
```

If remote auth or network is restricted:

```bash
CCR_FORCE_BUNDLE=1 claude --remote "task test TEST_FILE=spec/path/to/file_spec.rb"
```

## Move from web to terminal

From web:

```text
/teleport
```

From terminal:

```bash
claude --teleport
claude --teleport "$CLAUDE_CODE_REMOTE_SESSION_ID"
```

## Remote-friendly prep

- `service postgresql start`
- `service redis-server start`
- `docker compose up`

## Optional checks

- confirm current branch export:

```bash
echo "$MEDTRACKER_REMOTE_BRANCH"
```

- open transcript link:

```bash
echo "$MEDTRACKER_REMOTE_SESSION_URL"
```

- keep branch clean before teleport:

```bash
git status --short
```
