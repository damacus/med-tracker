# MedTracker

MedTracker is a Rails application for safe medication tracking across
prescriptions and non-prescription medicines, with auditability and care-team
support.

## Key capabilities

- Prescription and person-medicine tracking
- Dose recording with timing and daily-limit safeguards
- Carer-to-dependent relationship support
- Role-based access control
- Audit trail for safety-critical changes

## Stack

- Ruby on Rails
- PostgreSQL
- Hotwire (Turbo + Stimulus) + Phlex
- RSpec + Capybara/Playwright
- Docker Compose + Taskfile workflows

## Quick start

```bash
git clone https://github.com/damacus/med-tracker.git
cd med-tracker
task dev:up
task dev:seed
```

Open <http://localhost:3000>.

## Testing

```bash
task test
```

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
