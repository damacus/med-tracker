# Technical Quick Start

This guide is for developers and advanced users who want to run the full
MedTracker stack locally using Docker.

## Prerequisites

Before you begin, make sure you have these tools installed:
- [Docker](https://www.docker.com/) and Docker Compose
- [Task](https://taskfile.dev/) (the task runner for this project)
- Git

---

## 1. Clone the project

```bash
git clone https://github.com/damacus/med-tracker.git
cd med-tracker
```

## 2. Start development services

Run this command to start the database, web server, and worker:

```bash
task dev:up
```

*This command uses the configuration in `docker-compose.dev.yml`.*

## 3. Seed development data

To quickly populate the database with example users, people, and medicines:

```bash
task dev:seed
```

*The default password for all example users (e.g., `admin@example.com`) is `password`.*

## 4. Open the app

The MedTracker UI is available at:

👉 **[http://localhost:3000](http://localhost:3000)**

---

## Day-to-Day Development Commands

| Command | Action |
| --- | --- |
| `task dev:logs` | View real-time application logs |
| `task dev:stop` | Stop all development containers |
| `task dev:ps` | List running containers |
| `task test` | Run the full test suite in Docker |

## Troubleshooting

### Rebuild from scratch
If you encounter database issues or want to start fresh:
```bash
task dev:rebuild
```
*Warning: This removes all data and recreates the database.*

### Database migrations
To apply new database changes without a full rebuild:
```bash
task dev:run-migrations
```
