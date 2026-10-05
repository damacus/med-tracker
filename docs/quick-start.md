# Technical Quick Start

The root application is now Loco 1.2. Use a dedicated PostgreSQL 18 database,
set `DATABASE_URL`, run `task build`, then `task dev`. The foundation exposes
`/up`, database readiness at `/_health` and a Tera landing page. Product
workflows remain under migration; the Rails commands below run the reference
and rollback application under `rails/`. See the
[migration plan](plans/loco-migration-20261005/plan.md) for acceptance gates.

This guide is for developers and advanced users who want to run the full
MedTracker stack locally using Docker.

## Prerequisites

Before you begin, make sure you have these tools installed:
- [Docker](https://www.docker.com/) and Docker Compose
- [Task](https://taskfile.dev/) (the task runner for this project)
- [Portless](https://portless.sh/) (`npm install -g portless`, then run `portless trust` once)
- Git

---

## 1. Clone the project

```fish
git clone https://github.com/damacus/med-tracker.git
cd med-tracker
```

## 2. Start development services

Run this command to start the database, web server, and worker:

```fish
task rails:dev:portless
```

This starts the `dev` profile from `compose.yaml` and registers the stable local
URL `https://med-tracker.localhost`.

## 3. Seed development data

To quickly populate the database with example users, people, and medicines:

```fish
task rails:dev:seed
```

> **Local development only:** fixture data includes sample accounts with known
> passwords. Do not expose a seeded development stack to a public or shared
> network, and remove or reset sample accounts before using any real records.

## 4. Open the app

The MedTracker UI is available at:

👉 **[https://med-tracker.localhost](https://med-tracker.localhost)**

---

## Day-to-Day Development Commands

| Command | Action |
| --- | --- |
| `task rails:dev:logs` | View real-time application logs |
| `task rails:dev:stop` | Stop all development containers |
| `task rails:dev:ps` | List running containers |
| `task rails:test` | Run the full test suite in Docker |

## Troubleshooting

### Rebuild from scratch
If you encounter database issues or want to start fresh:
```fish
task rails:dev:rebuild
```
**Warning**

This command removes all data and recreates the database.

### Database migrations
To apply new database changes without a full rebuild:
```fish
task rails:dev:db-migrate
```
