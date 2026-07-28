# MedTracker

[![CI](https://github.com/damacus/med-tracker/actions/workflows/ci.yml/badge.svg)](https://github.com/damacus/med-tracker/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/damacus/med-tracker)](https://github.com/damacus/med-tracker/releases)
[![License](https://img.shields.io/github/license/damacus/med-tracker)](LICENSE)

MedTracker is an open-source, self-hosted medication tracker for individuals,
families, and carers. It supports household medication schedules, dose
recording, stock tracking, reminders, and auditable history.

> [!IMPORTANT]
> MedTracker is currently in beta. It should supplement—not replace—your
> existing medication routine. Do not depend on it for clinical decisions,
> emergency information, or your sole medication reminders.

![MedTracker dashboard showing today's medication schedule, dose status, and stock](docs/screenshots/dashboard-desktop.png)

## What MedTracker helps with

- Keep household medication schedules in one place
- Record doses with timing and daily-limit safeguards
- Track stock and see when supplies need attention
- Support children, dependent adults, and other people you care for
- Keep an attributable history of medication activity
- Control access with household roles and person-level permissions

## Try the self-hosted beta

We are looking for technically confident self-hosters who are willing to deploy
MedTracker, try the journey from household setup to recording doses, and tell us
where it is confusing or unreliable.

For a private local evaluation:

```shell
git clone https://github.com/damacus/med-tracker.git
cd med-tracker
task dev:up
task dev:seed
```

Open <http://localhost:3000>. Development seed data contains sample accounts
with known passwords, so never expose a seeded development instance to a public
or shared network.

Read the [self-hosting guide](https://damacus.github.io/med-tracker/self-hosting/)
before starting, and use the
[deployment guide](https://damacus.github.io/med-tracker/deployment/) for a
production-style installation.

## Share feedback

GitHub is the main home for MedTracker feedback:

- [Start a Discussion](https://github.com/damacus/med-tracker/discussions) for
  questions, early impressions, and self-hosting help
- [Report a bug or request a feature](https://github.com/damacus/med-tracker/issues/new/choose)
  using the short guided forms
- [Report a security vulnerability privately](https://github.com/damacus/med-tracker/security/advisories/new)

Please do not include names, medication details, health information, credentials,
tokens, or unredacted logs in public issues or discussions.

## Technology

- Ruby on Rails and PostgreSQL
- Hotwire, Phlex, RubyUI, and Tailwind CSS
- RSpec, Capybara, and Playwright
- Docker and Taskfile workflows

Run the project checks with `task test`, `task rubocop`, and `task brakeman`.

## Client Tools

First-party Rust tools live under `client-tools/`:

- `medtracker`: CLI for `/api/v1` workflows.
- `medtracker-mcp`: stdio MCP server for agent clients.

Run local tool gates with `task client-tools:fmt`,
`task client-tools:check`, `task client-tools:clippy`, and
`task client-tools:test`.

## Documentation

Published docs: <https://damacus.github.io/med-tracker/>

Key pages:

- [Quick Start](https://damacus.github.io/med-tracker/quick-start/)
- [Glossary](docs/glossary.md)
- [LLM Context Index (llms.txt)](https://damacus.github.io/med-tracker/llms.txt)
- [Kubernetes User Seeding](https://damacus.github.io/med-tracker/kubernetes-user-seeding/)
- [Carer Onboarding: First Dose](https://damacus.github.io/med-tracker/user-onboarding-carer-first-dose/)
- [Testing](https://damacus.github.io/med-tracker/testing/)
- [Client Tools](docs/api/client-tools.md)
- [Design](https://damacus.github.io/med-tracker/design/)
- [User Management](https://damacus.github.io/med-tracker/user-management/)

### Build docs locally

```bash
pip install -r requirements.txt
task docs:serve
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup instructions, development
workflow, and coding standards.
