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

## Development workflow

```bash
task dev:up       # start dev server (Docker)
task test         # run RSpec tests in Docker
task rubocop      # lint Ruby
task lighthouse:run  # accessibility/perf audit (requires dev:up)
task docs:serve   # serve docs locally
```

## Stack

- **Ruby on Rails** 8.1 — app server (runs in Docker)
- **PostgreSQL** — database (runs in Docker)
- **Node.js** — Lighthouse audits, Playwright e2e tests
- **Python/uv** — docs tooling (`zensical`)
- **Task** — task runner (`Taskfile.yml`)
