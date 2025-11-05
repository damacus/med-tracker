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

- **Backend:** Ruby on Rails 8.x
- **Database:** SQLite3 (for development)
- **Testing:** RSpec
- **Frontend:** Hotwire (Turbo, Stimulus), Phlex views
- **Observability:** OpenTelemetry for distributed tracing and monitoring

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

## Observability with OpenTelemetry

MedTracker includes OpenTelemetry instrumentation for distributed tracing and observability. This helps monitor application performance, debug issues, and understand system behavior.

### Quick Start with Jaeger (Local Development)

1. **Start Jaeger using Docker:**

   ```bash
   docker run -d --name jaeger \
     -e COLLECTOR_OTLP_ENABLED=true \
     -p 16686:16686 \
     -p 4318:4318 \
     jaegertracing/all-in-one:latest
   ```

2. **Configure environment variables** (optional, defaults work with above setup):

   ```bash
   cp opentelemetry.env.example .env
   # Edit .env if needed
   ```

3. **Run the application:**

   ```bash
   rails server
   ```

4. **View traces:**
   
   Open http://localhost:16686 in your browser to access the Jaeger UI.

### Configuration

OpenTelemetry is configured in `config/initializers/opentelemetry.rb`. Key environment variables:

- `OTEL_SERVICE_NAME`: Service name (default: `med-tracker`)
- `OTEL_SERVICE_VERSION`: Service version (default: `1.0.0`)
- `OTEL_EXPORTER_OTLP_ENDPOINT`: OTLP endpoint (default: `http://localhost:4318`)

See `opentelemetry.env.example` for all configuration options and `docs/opentelemetry_research.md` for detailed documentation.

### Disabling OpenTelemetry

OpenTelemetry is automatically disabled in the test environment. To disable it in other environments:

```bash
export OTEL_TRACES_EXPORTER=none
```

## Running the Tests

To run the full test suite, use the following command:

```bash
bundle exec rspec
```
