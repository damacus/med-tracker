# Med-Tracker

[![CI](https://github.com/damacus/med-tracker/actions/workflows/ci.yml/badge.svg)](https://github.com/damacus/med-tracker/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/damacus/med-tracker)](https://github.com/damacus/med-tracker/releases)
[![License](https://img.shields.io/github/license/damacus/med-tracker)](LICENSE)

Med-Tracker is an open-source, self-hosted medication tracker for individuals,
families, and carers. It supports household medication schedules, dose
recording, stock tracking, reminders, and auditable history.

> [!IMPORTANT]
> Med-Tracker is currently in beta. It should supplement your
> existing medication routine. Do not depend on it for clinical decisions,
> emergency information, or your sole medication reminders, and do not use it
> as a replacement for established care.

![Med-Tracker dashboard showing today's medication schedule, dose status, and stock](docs/screenshots/dashboard-desktop.png)

## Why Med-Tracker?

Medication management is rarely just a list of pills. When care is shared,
families need to know what is due, what has already happened, whether supplies
are running low, and who recorded each action. Med-Tracker brings that context
together without handing control of it to a hosted service.

- See today's medication routine for everyone you care for in one place
- Replace uncertainty about a dose with a clear history
- Spot low stock before it becomes a last-minute problem
- Coordinate care for children, dependent adults, and other household members
- Keep an attributable record of who did what and when
- Run Med-Tracker on infrastructure you control

## Migration checkout

This branch makes Loco 1.2 the root Rust application and retains Rails under
`rails/` as read-only reference code. The foundation currently exposes
`/up`, framework database readiness at `/_health`, and a Tera landing page.
The accepted fresh Rust architecture is recorded in
[ADRs 0013–0016](docs/design.md#accepted-rust-direction). These decisions do not
certify implementation acceptance.

Use `task deps` once to resolve dependencies, `task build` to compile and `task ci`
to verify Loco. Set `DATABASE_URL` to a dedicated PostgreSQL 18 database and run
`task dev`. Startup does not migrate, truncate or recreate the product schema.
`task db:migrate MIGRATION_DATABASE_URL=...` creates the complete schema in an
empty database. `task db:prepare SEED_DATABASE_URL=...` also loads synthetic
development fixtures. These commands use privileged setup credentials; the
application uses its restricted runtime role. Existing database adoption is not
supported. `task release-image` remains guarded. Do not point foundation
verification at a live database. See [database setup](docs/testing.md#loco-database-setup).

## Try the Rails self-hosted beta

We are looking for technically confident self-hosters to run Med-Tracker and try
the journey from household setup to recording doses. Tell us where the app is
confusing or unreliable.

For a private local evaluation:

```shell
git clone https://github.com/damacus/med-tracker.git
cd med-tracker
npm install -g portless
portless trust
task rails:dev:portless
task rails:dev:seed
```

Open <https://med-tracker.localhost>. Development seed data contains sample
accounts with known passwords, so never expose a seeded development instance
to a public or shared network.

Read the [self-hosting guide](https://damacus.github.io/med-tracker/self-hosting/)
before starting, and use the
[deployment guide](https://damacus.github.io/med-tracker/deployment/) for a
production-style installation.

## Share feedback

GitHub is the main home for Med-Tracker feedback:

- [Start a Discussion](https://github.com/damacus/med-tracker/discussions) for
  questions, early impressions, and self-hosting help
- [Report a bug or request a feature](https://github.com/damacus/med-tracker/issues/new/choose)
  using the short guided forms
- [Report a security vulnerability privately](https://github.com/damacus/med-tracker/security/advisories/new)

Please do not include names, medication details, health information, credentials,
tokens, or unredacted logs in public issues or discussions.

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
- [Record a First Dose](docs/families/taking-first-dose.md)
- [Testing](https://damacus.github.io/med-tracker/testing/)
- [Client Tools](docs/api/client-tools.md)
- [Design](https://damacus.github.io/med-tracker/design/)
- [User Management](https://damacus.github.io/med-tracker/user-management/)

### Build docs locally

```fish
task docs:serve
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup instructions, development
workflow, and coding standards.
