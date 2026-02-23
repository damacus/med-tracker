# MedTracker Agent Guide

> `CLAUDE.md` is a symlink to this file (`agents.md`). Edit `agents.md`.

## Non-obvious rules

- **Shell** — Fish syntax only (`set VAR value`, `(cmd)`, `if … end`)
- **Comments** — Never add or remove comments unless explicitly asked
- **Person enum** — `adult:0`, `minor:1`, `dependent_adult:2`; minors/dependent_adults always `has_capacity:false`
- **PostgreSQL 18** — Use version 18, not 17, in all configs and docs
- **Fixture password** — All dev/test fixture users have password `password`

## Commands

Use `task` for everything. Never run `docker compose`, `bin/dev`, or `bundle exec rspec` directly.

| What | Command |
|---|---|
| Run tests | `task test` |
| Lint | `task rubocop` |
| Start dev server | `task dev:up` |
| Stop dev server | `task dev:stop` |
| Get dev port | `task dev:port` |
| Open in browser | `task dev:open-ui` |
| Seed database | `task dev:seed` |
| Migrate | `task dev:db-migrate` |
| Rebuild (destructive) | `task dev:rebuild` |
| Stop everything | `task stop-all` |
| List all tasks | `task -l` |

## Issue tracking

Use **Beads** (`bd`) for all tasks and issues. Never use TodoWrite or markdown task files.

```
bd ready               # find available work
bd create --title="…"  # create issue before writing code
bd update <id> --status=in_progress
bd close <id>
bd sync                # sync with remote (run at session end)
```

## Screenshots for PRs

```bash
task dev:up
task dev:port          # → e.g. 3000
# then use the playwright-cli skill to navigate and screenshot
```

## Session close (mandatory)

```bash
git pull --rebase
bd sync
git push
```

Work is not done until `git push` succeeds.
