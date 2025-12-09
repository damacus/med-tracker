# MedTracker

A simple and effective application to help you manage and track your medication schedule.

## Overview

MedTracker is a Ruby on Rails application designed to help users monitor their medication intake, ensuring they adhere to their prescribed schedules and restrictions. It provides a clear interface for managing prescriptions and logging doses, with built-in validations to prevent common mistakes like taking too much medication or taking doses too close together.

## Features

- **Prescription Management:** Create and manage prescriptions with details like dosage, frequency, and start/end dates.
- **Dose Tracking:** Easily log each dose of medication taken.
- **Timing Restrictions:** Set rules for maximum daily doses and minimum hours between doses to prevent accidental overdose.
- **Dose Cycles:** Supports daily, weekly, and monthly dosing schedules.
- **Active/Inactive Prescriptions:** Automatically tracks which prescriptions are currently active.
- **Validation:** Smart validations to ensure doses are taken according to prescription rules.

## Tech Stack

- **Backend:** Ruby on Rails
- **Database:** PostgreSQL
- **Testing:** RSpec + Capybara
- **Frontend:** Hotwire (Turbo, Stimulus) + Phlex
- **Deployment:** Docker Compose

## Getting Started

Ready to start using MedTracker? Check out our comprehensive [Quick Start Guide](https://damacus.github.io/med-tracker/quick-start/) for detailed setup instructions.

### Quick Setup

```bash
# Clone the repository
git clone https://github.com/damacus/med-tracker.git
cd med-tracker

# Install dependencies
bundle install

# Setup database (requires PostgreSQL)
rails db:create db:migrate

# Start the application
bin/dev
```

Visit `http://localhost:3000` to access the application.

For Docker deployment, environment variables, and troubleshooting, see the [Quick Start Guide](https://damacus.github.io/med-tracker/quick-start/).

## Running Tests

```bash
bundle exec rspec
```

For comprehensive testing information including Lighthouse audits, see the [Testing Guide](https://damacus.github.io/med-tracker/testing/).

## Deployment

MedTracker supports multiple deployment options:

- **Docker Compose** - Recommended for most deployments
- **Kamal** - Zero-downtime deployments
- **Traditional VPS** - Standard Rails deployment

See the [Deployment Guide](https://damacus.github.io/med-tracker/deployment/) for detailed instructions.

## Documentation

Comprehensive documentation is available at **[https://damacus.github.io/med-tracker/](https://damacus.github.io/med-tracker/)**

Key documentation sections:

- **[Quick Start Guide](https://damacus.github.io/med-tracker/quick-start/)** - Setup instructions and prerequisites
- **[Deployment Guide](https://damacus.github.io/med-tracker/deployment/)** - Docker and production deployment
- **[Testing Guide](https://damacus.github.io/med-tracker/testing/)** - Running tests and Lighthouse audits
- **[Design & Architecture](https://damacus.github.io/med-tracker/design/)** - Technical decisions and data models
- **[User Management](https://damacus.github.io/med-tracker/user-management/)** - Roles, permissions, and capacity management

### Building Documentation Locally

To view the documentation locally:

```bash
# Install dependencies
pip install -r requirements.txt

# Serve the documentation
mkdocs serve
```

Open your browser to `http://localhost:8000`.

### Documentation Agent

The project includes a GitHub Copilot agent that can help identify and
create documentation opportunities. See `.github/agents/README.md` for
more information.
