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
- **Database:** SQLite3 (for development)
- **Testing:** Minitest
- **Frontend:** Hotwire (Turbo, Stimulus)

## Getting Started

### Prerequisites

- Ruby
- Bundler
- Node.js

### Setup

1. **Clone the repository:**

   ```bash
   git clone https://github.com/damacus/med-tracker.git
   cd med-tracker
   ```

2. **Install dependencies:**

   ```bash
   bundle install
   ```

3. **Set up the database:**

   ```bash
   rails db:create
   rails db:migrate
   ```

4. **Run the application:**

   ```bash
   rails server
   ```

   Open your browser to `http://localhost:3000`.

## Running the Tests

To run the full test suite, use the following command:

```bash
bundle exec rake test
```

## Documentation

Comprehensive documentation is available in the `docs/` directory:

- [Design & Architecture](docs/design.md)
- [User Management System](docs/user-management.md)

### Building Documentation

This project uses mkdocs for documentation. To view the documentation locally:

1. Install mkdocs:

   ```bash
   pip install mkdocs mkdocs-material
   ```

2. Serve the documentation:

   ```bash
   mkdocs serve
   ```

3. Open your browser to `http://localhost:8000`

### Documentation Agent

The project includes a GitHub Copilot agent that can help identify and
create documentation opportunities. See `.github/agents/README.md` for
more information.
