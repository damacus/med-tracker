# Technical Quick Start

Run the root Rust/Loco application for local development. Rails under `rails/`
is read-only reference. Read the [0.6 release notes](releases/0.6.md) before
setting up a new installation.

There is no supported rollback from 0.6 to Rails. A full Rails export at the
transition is an operator action, not an automatic import or a claim that an
export has already happened. Any retained Rails site contains only its
pre-cutover state. Preserve data written in 0.6 with a separate Rust-side export;
returning to the old site would not carry those records back. These notes do
not promise that Rails can import that export or provide a lossless switchback.

## Prerequisites

Install Git and [mise](https://mise.jdx.dev/), then clone the repository:

```fish
git clone https://github.com/damacus/med-tracker.git
cd med-tracker
mise trust
mise install
```

The committed `mise.toml` selects the tools. A tool installation failure is a
setup failure, not an application test result. Docker is needed for the
repository's disposable integration and browser test services. Local development
also needs PostgreSQL 18 and the native libraries used by the Rust build; see
the repository Taskfile for the platform-specific ICU configuration.

## Prepare a private development database

Create a new empty PostgreSQL 18 database. Keep setup credentials separate from
the restricted application login. Follow the role and extension requirements in
[Loco database setup](testing.md#loco-database-setup).

Use `task db:migrate MIGRATION_DATABASE_URL=...` for schema installation, or
`task db:prepare SEED_DATABASE_URL=...` to install the schema and synthetic
development fixtures. Replace `...` with your private setup connection URL.
Do not use either path to adopt an existing Rails database. Seeding rejects
populated application databases and production environments.

Fixture passwords are `password`. Keep a seeded development instance private;
do not use synthetic accounts for real medication records.

## Start the application

Set `DATABASE_URL` to your restricted runtime connection, then run:

```fish
task build
task dev
```

`task dev` prepares the frontend assets and starts Loco in development mode.
Use the address printed by the server. Development configuration is in
`config/development.yaml`; production settings and secrets require the separate
[deployment guide](deployment.md).

If background processing is needed, run `task worker` in another terminal. That
Task also starts a scheduler; do not start `task scheduler` alongside it.

## Check changes

Use `task test:unit` for database-free checks, `task test:integration` for API and
database behaviour, and `task test:browser` for Rust Playwright journeys.
`task ci` runs the application checks. See the [testing guide](testing.md)
for isolated databases, focused filters and screenshots.

## Troubleshooting

Check the server output, database connection and installed tools first. Do not
reset a populated database as a troubleshooting step. Create a separate empty
development database when you need a fresh fixture environment. Rails commands
do not start or repair the root Rust application.
