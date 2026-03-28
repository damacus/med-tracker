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

> **Note**: For most `task dev:*` commands, an equivalent `task test:*` command exists (e.g., `task test:up`, `task test:port`).

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

## Screenshots for PRs

```bash
task dev:up
task dev:port          # → e.g. 3000
# then use the playwright-cli skill to navigate and screenshot
```

## Quality gates (run before every push)

```bash
task rubocop          # lint — must pass with no offenses
task test             # full test suite in Docker — must be green
```

Run a single file during development:

```bash
task test TEST_FILE=spec/path/to/file_spec.rb
```

Never push if either command fails.

## Session close (mandatory)

```bash
git pull --rebase
git push
```

Work is not done until `git push` succeeds.

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
