#!/bin/bash
set -euo pipefail

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$PROJECT_DIR"

POLICY_CACHE_DIR="${PROJECT_DIR}/.claude/session-cache"
mkdir -p "$POLICY_CACHE_DIR"

POLICY_DONE_FILE="${POLICY_CACHE_DIR}/session-policies.done"
if [ -f "$POLICY_DONE_FILE" ]; then
  exit 0
fi

if command -v git >/dev/null 2>&1; then
  ACTIVE_BRANCH="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || true)"
else
  ACTIVE_BRANCH=""
fi

if [ -n "${CLAUDE_ENV_FILE:-}" ]; then
  {
    echo "export MEDTRACKER_REMOTE_SESSION_ACTIVE=true"
    if [ -n "$ACTIVE_BRANCH" ]; then
      echo "export MEDTRACKER_REMOTE_BRANCH=\"$ACTIVE_BRANCH\""
    fi
    echo "export MEDTRACKER_REMOTE_SESSION_URL=\"https://claude.ai/sessions/${CLAUDE_CODE_REMOTE_SESSION_ID}\""
  } >> "$CLAUDE_ENV_FILE"
fi

touch "$POLICY_DONE_FILE"
