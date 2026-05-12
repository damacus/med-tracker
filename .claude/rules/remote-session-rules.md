# MedTracker Claude Remote Session Rules

## Session defaults

- Use `claude --remote "<task>"` to start cloud sessions.
- Run cloud-safe heavy prep only on explicit setup hooks.
- Keep `.env` values in `KEY=value` form for easy replay in sessions.
- Prefer default allowed domains first; add only targeted exceptions.

## Security and data handling

- Never place long-lived secrets in `.env` or command snippets copied into chat.
- Use cloud session environment variables only for ephemeral values.
- Verify local secrets are scoped to your trusted terminal session.

## Teleport expectations

- working tree should be clean before transfer
- local branch and remote branch should match
- branch must exist on remote
- same `claude.ai` account should be used in both contexts
- keep branch history aligned before continuing in terminal

## Optional prechecks

- `service postgresql start`
- `service redis-server start`
- `docker compose up` (project services only when required)

## Required hook output for this repo

- Session exports:
  - `MEDTRACKER_REMOTE_SESSION_ACTIVE=true`
  - `MEDTRACKER_REMOTE_BRANCH`
  - `MEDTRACKER_REMOTE_SESSION_URL`
