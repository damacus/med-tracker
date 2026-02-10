# Quick Start

This guide gets MedTracker running locally with the project-standard
`task` commands.

## Prerequisites

- Docker and Docker Compose
- [Task](https://taskfile.dev/)
- Git

## 1. Clone the project

```bash
git clone https://github.com/damacus/med-tracker.git
cd med-tracker
```

## 2. Start development services

```bash
task dev:up
```

This starts the development stack defined in `docker-compose.dev.yml`.

## 3. Seed development data

```bash
task dev:seed
```

The seed process loads fixtures from `spec/fixtures/` via `db/seeds.rb`.
Default fixture user passwords are `password`.

## 4. Open the app

Visit <http://localhost:3000>.

## Common day-to-day commands

```bash
task dev:logs
task dev:stop
task dev:ps
```

## Run tests

Use the test environment through Taskfile commands:

```bash
task test
```

For local CI-like runs outside Dockerized app services:

```bash
task local:test
task local:test:browser
task local:test:all
```

## Troubleshooting

### Rebuild from scratch

```bash
task dev:rebuild
```

This removes volumes and recreates the development database.

### Database migration updates

```bash
task dev:run-migrations
```

## Next guides

- [Carer Onboarding: First Dose](user-onboarding-carer-first-dose.md)
- [Testing](testing.md)
- [Deployment](deployment.md)
