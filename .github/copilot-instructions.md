# GitHub Copilot Instructions for MedTracker

`AGENTS.md` is the authoritative repository guide. Follow its architecture,
testing, security, UI, data-access, command, and verification rules.

## Orientation

- Read `README.md` for the product and local setup.
- Read `docs/design.md` for the current architecture and domain records.
- Use `docs/glossary.md` for medication and care terminology.
- Resolve current versions from `.ruby-version`, `Gemfile.lock`, and
  `compose/base.yml`; do not copy version numbers from old plans.

## Repository contracts

- Use `task` commands for development, tests, linting, security checks, and
  container workflows. Do not run raw Rails, RSpec, RuboCop, Brakeman, or
  Docker Compose commands.
- Start local development with `task dev:portless` and use
  `https://med-tracker.localhost`.
- Use PostgreSQL in every environment.
- Views are Phlex components under `app/components/`; do not create ERB views.
- Authentication uses Rodauth with passwords, passkeys, and OIDC.
- Authorization uses household memberships, person access grants, and Pundit
  policies.
- Follow Red-Green-Refactor for executable behavior changes.
- For Markdown-only changes, run `task docs:build` and `git diff --check`
  instead of the full application suite.

## Safety

- Treat medication, person, household, authentication, and audit data as
  sensitive.
- Preserve authorization boundaries and attributable medication history.
- Never add real health information, credentials, tokens, or unredacted logs
  to fixtures, documentation, issues, or commits.
