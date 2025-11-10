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

### Prerequisites

- Ruby
- Bundler
- Node.js
- PostgreSQL (or Docker)

### Setup

1. **Clone the repository:**

   ```bash
   git clone https://github.com/damacus/med-tracker.git
   cd med-tracker
   ```

2. **Start PostgreSQL:**

   Using Docker (recommended):

   ```bash
   docker-compose -f docker-compose.dev.yml up -d
   ```

   Or use your local PostgreSQL installation.

3. **Install dependencies:**

   ```bash
   bundle install
   ```

4. **Set up the database:**

   ```bash
   rails db:create
   rails db:migrate
   ```

5. **Run the application:**

   ```bash
   rails server
   ```

   Open your browser to `http://localhost:3000`.

### Environment Variables

The following environment variables can be configured:

- `DB_HOST` - PostgreSQL host (default: localhost)
- `DB_USERNAME` - PostgreSQL username (default: postgres)
- `DB_PASSWORD` - PostgreSQL password (default: postgres)

## Running the Tests

To run the full test suite, use the following command:

```bash
bundle exec rspec
```

## Docker Deployment

MedTracker includes Docker Compose configurations for development, production, and test environments.

### Quick Start

**Development (with live code reloading):**

```bash
# First time setup
cp .env.example .env
docker compose -f docker-compose.dev.yml up -d
./run db:setup

# Start the app
docker compose -f docker-compose.dev.yml up
```

Access the app at `http://localhost:3000`

**Run migrations:**

```bash
./run db:migrate
```

**Access Rails console:**

```bash
./run rails console
```

**Run tests:**

```bash
./run test
```

### Production Environment

```bash
# Requires config/credentials/production.key file
docker compose up -d
./run db:migrate
```

### Available Commands

The `./run` script provides convenient shortcuts:

- `./run rails <command>` - Run Rails commands
- `./run db:setup` - Setup database (first time)
- `./run db:migrate` - Run migrations
- `./run db:reset` - Reset database
- `./run shell` - Open bash shell in container
- `./run test` - Run RSpec tests
- `./run format` - Auto-fix RuboCop issues
- `./run lint` - Check code style
- `./run help` - Show all available commands
