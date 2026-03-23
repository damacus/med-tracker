#!/bin/bash
set -euo pipefail

# Only run in remote (web) sessions
if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

cd "${CLAUDE_PROJECT_DIR:-$(pwd)}"

# ── Ruby gems ────────────────────────────────────────────────────────────────
# Use bundler 2.x — bundler 4.x has a CGI bug that breaks `bundle install`
# when the locked bundler version differs from the installed one.
bundle _2.5.22_ install

# ── Node packages ────────────────────────────────────────────────────────────
npm install

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

install_cft_browser() {
  local dir="$1" subdir="$2" zip_name="$3" binary="$4"
  local dest="${PLAYWRIGHT_CACHE}/${dir}"
  if [ -f "${dest}/INSTALLATION_COMPLETE" ] && [ -x "${dest}/${subdir}/${binary}" ]; then
    return
  fi
  mkdir -p "${dest}"
  curl -fsSL "${CfT_BASE}/${zip_name}" -o /tmp/pw-browser.zip
  rm -rf "${dest:?}/${subdir}"
  unzip -q /tmp/pw-browser.zip -d /tmp/pw-extract/
  mv "/tmp/pw-extract/${subdir}" "${dest}/${subdir}"
  rm -rf /tmp/pw-extract /tmp/pw-browser.zip
  touch "${dest}/INSTALLATION_COMPLETE"
}

install_cft_browser "chromium-1208" \
  "chrome-linux64" "chrome-linux64.zip" "chrome"
install_cft_browser "chromium_headless_shell-1208" \
  "chrome-headless-shell-linux64" "chrome-headless-shell-linux64.zip" "chrome-headless-shell"

# ── PostgreSQL ───────────────────────────────────────────────────────────────
# Start the cluster if it is not already running
if ! pg_ctlcluster 16 main status &>/dev/null; then
  pg_ctlcluster 16 main start
fi

# Create the `root` superuser role (peer auth over Unix socket; idempotent)
psql -U postgres -c "CREATE ROLE root WITH SUPERUSER LOGIN;" 2>/dev/null || true

# ── Test database ────────────────────────────────────────────────────────────
psql -c "CREATE DATABASE medtracker_test OWNER root;" 2>/dev/null || true

DATABASE_URL=postgresql:///medtracker_test \
  RAILS_ENV=test \
  bundle _2.5.22_ exec rails db:migrate

# ── Precompile assets ────────────────────────────────────────────────────────
DATABASE_URL=postgresql:///medtracker_test \
  RAILS_ENV=test \
  bundle _2.5.22_ exec rails assets:precompile

# ── Persist DATABASE_URL for the session ────────────────────────────────────
echo 'export DATABASE_URL=postgresql:///medtracker_test' >> "${CLAUDE_ENV_FILE:-/dev/null}"
