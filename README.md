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

MedTracker includes Docker Compose configurations for both production and test environments. See [README.docker.md](README.docker.md) for detailed instructions.

**Quick Start:**

```bash
# Production environment
export RAILS_MASTER_KEY=$(cat config/master.key)
docker-compose up -d

# Test environment
docker-compose -f docker-compose.test.yml up
```
