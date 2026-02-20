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

## Documentation

Published docs: <https://damacus.github.io/med-tracker/>

Key pages:

- [Quick Start](https://damacus.github.io/med-tracker/quick-start/)
- [LLM Context Index (llms.txt)](https://damacus.github.io/med-tracker/llms.txt)
- [Kubernetes User Seeding](https://damacus.github.io/med-tracker/kubernetes-user-seeding/)
- [Carer Onboarding: First Dose](https://damacus.github.io/med-tracker/user-onboarding-carer-first-dose/)
- [Testing](https://damacus.github.io/med-tracker/testing/)
- [Design](https://damacus.github.io/med-tracker/design/)
- [User Management](https://damacus.github.io/med-tracker/user-management/)

### Build docs locally

```bash
pip install -r requirements.txt
task docs:serve
```
