# Claude Code — Project Guide

## Environment Setup

Each session starts with a fresh environment. Run the setup script before doing anything else:

```bash
bin/setup-claude
```

This installs:
- **`task`** CLI (downloaded from GitHub releases — `go-task/task`)
- **Ruby gems** via `bundle install`
- **npm packages** (`lighthouse`, `playwright`) via `npm install`
- **Python packages** (`zensical`) via `uv sync`
- **Playwright browsers** (chromium symlink workaround — see below)

### Known quirks

**Bundler 4.0.x + Ruby 3.3 + proxy**: Running `bundle install` crashes with
`uninitialized class variable @@accept_charset in CGI`. Fix: prefix with
`RUBYOPT="-rcgi"` so CGI is preloaded before bundler's vendored
net-http-persistent tries to use it. The setup script handles this.

**Playwright browser downloads**: `cdn.playwright.dev` is not reachable.
Chromium 1194 is pre-cached at `~/.cache/ms-playwright/chromium-1194`.
The setup script symlinks it to `chromium-1208` (what Playwright 1.58.2
expects). Tests using Playwright will run against a slightly older Chromium.

**`task` install**: `taskfile.dev` install script is blocked (403 on CONNECT).
The setup script downloads the pre-built binary directly from GitHub releases.

## Claude Code on the Web

`claude --remote "<task>"` starts a cloud-hosted assistant session.

Remote sessions are launched from the GitHub remote branch, not your local
working tree. Push local changes before starting a remote session if you need them
in the cloud copy.

If GitHub auth is unavailable in the remote flow, use:

```bash
CCR_FORCE_BUNDLE=1 claude --remote "task test TEST_FILE=spec/..._spec.rb"
```

### Session workflow

```bash
claude --remote "task test TEST_FILE=spec/..."
```

From web: run `/teleport`.

Back in terminal:

```bash
claude --teleport
claude --teleport "$CLAUDE_CODE_REMOTE_SESSION_ID"
```

### `--remote` vs `--remote-control`

`--remote` creates a new cloud session for the current task.
`--remote-control` is for controlling existing local sessions from the web
context and is not a replacement for cloud startup.

### Teleport prerequisites

- clean working tree
- same repository and branch checked out locally
- target branch pushed to remote
- same `claude.ai` account in browser and terminal
- session ID available in environment

### Cloud handoff helpers

- `service postgresql start`
- `service redis-server start`
- `docker compose up` (when local project services are required)
- `echo "https://claude.ai/sessions/$CLAUDE_CODE_REMOTE_SESSION_ID"`

### Cloud limits / gotchas

- no secret store exists in cloud sessions; session env vars are visible by scope
- sessions are fresh, while repo files are cached across sessions
- hooks run on startup/resume and can add startup latency
- `--teleport` depends on clean working tree and branch parity with remote

### Allowed domains and env format

- prefer default allowed domains first
- add custom domains only when needed
- `.env` values are standard `KEY=value` lines, for example:

```bash
DATABASE_URL=postgresql:///medtracker_test
REDIS_URL=redis://localhost:6379/0
FEATURE_FLAG=true
```

### Skills shipped with this repo

- [playwright-cli](/Users/damacus/repos/damacus/med-tracker/.claude/skills/playwright-cli/SKILL.md): browser/test tooling
- [claude-web-session](/Users/damacus/repos/damacus/med-tracker/.claude/skills/claude-web-session/SKILL.md): remote session handoff and teleport workflows
- [ruby](/Users/damacus/repos/damacus/med-tracker/.claude/skills/ruby/SKILL.md): Ruby language guidance and patterns
- [rails](/Users/damacus/repos/damacus/med-tracker/.claude/skills/rails/SKILL.md): Rails development and architecture
- [rspec](/Users/damacus/repos/damacus/med-tracker/.claude/skills/rspec/SKILL.md): test strategy, fixtures, and specs
- [rubocop](/Users/damacus/repos/damacus/med-tracker/.claude/skills/rubocop/SKILL.md): Ruby linting and autofix workflows
- [ui-professional](/Users/damacus/repos/damacus/med-tracker/.claude/skills/ui-professional/SKILL.md): UI quality and design principles for frontend work

### Session rules

See [remote-session-rules](/Users/damacus/repos/damacus/med-tracker/.claude/rules/remote-session-rules.md) for operational expectations.

## Development workflow

```bash
task dev:up       # start dev server (Docker)
COMPOSE_PROFILES=mailpit task dev:up  # start dev server with Mailpit for email testing
task local:test    # run non-browser tests locally (uses task local db container)
task test         # run RSpec tests in Docker
task rubocop      # lint Ruby
task lighthouse:run  # accessibility/perf audit (requires dev:up)
task docs:serve   # serve docs locally
```

## Cloud / no-Docker workflows

For environments without Docker, run Rails directly after preparing a local PostgreSQL 18 database and setting `DATABASE_URL`.

Run stack (without Docker):

```bash
export DATABASE_URL='postgres://medtracker:medtracker_password@localhost:5432/medtracker'
export RAILS_ENV=development
bundle exec rails db:prepare
bundle exec rails server -b 0.0.0.0 -p 3000
```

Run tests (without Docker):

```bash
export RAILS_ENV=test
export DATABASE_URL='postgres://medtracker:medtracker_password@localhost:5432/medtracker_test'
bundle exec rspec
bundle exec rspec spec/models/foo_spec.rb
```

For browser/system specs without Docker:

```bash
bundle exec rails assets:precompile
bundle exec rspec --tag browser
```

## Stack

- **Ruby on Rails** 8.1 — app server (runs in Docker)
- **PostgreSQL** — database (runs in Docker)
- **Node.js** — Lighthouse audits, Playwright e2e tests
- **Python/uv** — docs tooling (`zensical`)
- **Task** — task runner (`Taskfile.yml`)
