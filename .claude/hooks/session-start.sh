#!/bin/bash
set -euo pipefail

# Only run in remote (web) sessions
if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$PROJECT_DIR"

SESSION_CACHE_DIR="${PROJECT_DIR}/.claude/session-cache"
mkdir -p "$SESSION_CACHE_DIR"

# ── Ruby gems ────────────────────────────────────────────────────────────────
# Use bundler 2.x — bundler 4.x has a CGI bug that breaks `bundle install`
# when the locked bundler version differs from the installed one.
if command -v bundle >/dev/null 2>&1; then
  if ! bundle _2.5.22_ check >/tmp/bundle-check.log 2>&1; then
    bundle _2.5.22_ install >/tmp/bundle-install.log 2>&1 || true
  fi
else
  echo "warning: bundle not found; skipping Ruby dependency install." >&2
fi

# ── Node packages ────────────────────────────────────────────────────────────
if command -v npm >/dev/null 2>&1; then
  npm install --prefer-offline --no-audit --no-fund >/tmp/npm-install.log 2>&1 || true
else
  echo "warning: npm not found; skipping Node dependency install." >&2
fi

# ── Playwright browsers ──────────────────────────────────────────────────────
# The playwright CDN (cdn.playwright.dev) is blocked in this environment.
# Download Chrome for Testing directly from Google Storage instead.
#
# playwright 1.58.2 expects build v1208 (Chrome 145.0.7632.6) at:
#   chromium-1208/chrome-linux64/chrome
#   chromium_headless_shell-1208/chrome-headless-shell-linux64/chrome-headless-shell
CHROME_VERSION="145.0.7632.6"
PLAYWRIGHT_CACHE="/root/.cache/ms-playwright"
CfT_BASE="https://storage.googleapis.com/chrome-for-testing-public/${CHROME_VERSION}/linux64"
PLAYWRIGHT_MARKER="${SESSION_CACHE_DIR}/playwright-browsers-ok"

install_cft_browser() {
  local dir="$1" subdir="$2" zip_name="$3" binary="$4"
  local dest="${PLAYWRIGHT_CACHE}/${dir}"
  if [ -f "${dest}/INSTALLATION_COMPLETE" ] && [ -x "${dest}/${subdir}/${binary}" ]; then
    return
  fi
  if [ ! -d "${PLAYWRIGHT_CACHE}" ]; then
    mkdir -p "${PLAYWRIGHT_CACHE}"
  fi
  if [ -f "${dest}/INSTALLATION_COMPLETE" ] && [ -x "${dest}/${subdir}/${binary}" ]; then
    return
  fi
  mkdir -p "${dest}"
  if [ ! -f /tmp/pw-browser.zip ]; then
    rm -f /tmp/pw-browser.zip
  fi
  if ! command -v curl >/dev/null 2>&1 || ! command -v unzip >/dev/null 2>&1; then
    echo "warning: curl/unzip missing; skipping Playwright browser install." >&2
    return
  fi
  curl -fsSL "${CfT_BASE}/${zip_name}" -o /tmp/pw-browser.zip || return
  rm -rf "${dest:?}/${subdir}"
  unzip -q /tmp/pw-browser.zip -d /tmp/pw-extract/ || return
  mv "/tmp/pw-extract/${subdir}" "${dest}/${subdir}"
  rm -rf /tmp/pw-extract /tmp/pw-browser.zip
  touch "${dest}/INSTALLATION_COMPLETE"
}

if [ ! -f "$PLAYWRIGHT_MARKER" ]; then
  install_cft_browser "chromium-1208" \
    "chrome-linux64" "chrome-linux64.zip" "chrome" || true
  install_cft_browser "chromium_headless_shell-1208" \
    "chrome-headless-shell-linux64" "chrome-headless-shell-linux64.zip" "chrome-headless-shell" || true
  if [ -x "${PLAYWRIGHT_CACHE}/chromium-1208/chrome-linux64/chrome" ] && \
     [ -x "${PLAYWRIGHT_CACHE}/chromium_headless_shell-1208/chrome-headless-shell-linux64/chrome-headless-shell" ]; then
    touch "$PLAYWRIGHT_MARKER"
  fi
fi

# ── PostgreSQL ───────────────────────────────────────────────────────────────
# Start the cluster if it is not already running
if command -v service >/dev/null 2>&1; then
  service postgresql start || true
fi
if command -v pg_lsclusters >/dev/null 2>&1; then
  PG_CLUSTER_VERSION="$(pg_lsclusters 2>/dev/null | awk '$2=="main"{print $1; exit}')"
  if [ -n "${PG_CLUSTER_VERSION:-}" ] && ! pg_ctlcluster "${PG_CLUSTER_VERSION}" main status &>/dev/null; then
    pg_ctlcluster "${PG_CLUSTER_VERSION}" main start || true
  fi
fi

# Create the `root` superuser role (peer auth over Unix socket; idempotent)
if command -v psql >/dev/null 2>&1; then
  psql -U postgres -c "CREATE ROLE root WITH SUPERUSER LOGIN;" 2>/dev/null || true

  # ── Test database ────────────────────────────────────────────────────────────
  psql -c "CREATE DATABASE medtracker_test OWNER root;" 2>/dev/null || true

  if command -v bundle >/dev/null 2>&1 && [ ! -f "${SESSION_CACHE_DIR}/db-migrate.done" ]; then
    DATABASE_URL=postgresql:///medtracker_test \
      RAILS_ENV=test \
      bundle _2.5.22_ exec rails db:migrate && touch "${SESSION_CACHE_DIR}/db-migrate.done" || true
  fi

  # ── Precompile assets ────────────────────────────────────────────────────────
  if [ ! -f "${SESSION_CACHE_DIR}/assets-precompile.done" ]; then
    DATABASE_URL=postgresql:///medtracker_test \
      RAILS_ENV=test \
      bundle _2.5.22_ exec rails assets:precompile && touch "${SESSION_CACHE_DIR}/assets-precompile.done" || true
  fi
fi

# ── Persist DATABASE_URL for the session ────────────────────────────────────
echo 'export DATABASE_URL=postgresql:///medtracker_test' >> "${CLAUDE_ENV_FILE:-/dev/null}"
