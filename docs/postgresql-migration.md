# PostgreSQL Migration Guide

MedTracker now uses PostgreSQL for all environments (development, test, and production).

## Quick Start

### Using Docker (Recommended)

1. Start PostgreSQL:

   ```bash
   docker-compose -f docker-compose.dev.yml up -d
   ```

2. Install dependencies:

   ```bash
   bundle install
   ```

3. Set up the database:

   ```bash
   rails db:create
   rails db:migrate
   ```

### Using Local PostgreSQL

1. Install PostgreSQL (if not already installed):

   ```bash
   # macOS
   brew install postgresql@17

   # Ubuntu/Debian
   sudo apt-get install postgresql-17
   ```

2. Start PostgreSQL service

3. Create a postgres user (if needed):

   ```bash
   createuser -s postgres
   ```

4. Set environment variables (optional):

   ```bash
   export DB_HOST=localhost
   export DB_USERNAME=postgres
   export DB_PASSWORD=postgres
   ```

5. Install dependencies and set up database:

   ```bash
   bundle install
   rails db:create
   rails db:migrate
   ```

## Configuration

Database configuration is in `config/database.yml`. The following environment variables can be set:

- `DB_HOST` - PostgreSQL host (default: localhost)
- `DB_USERNAME` - PostgreSQL username (default: postgres)
- `DB_PASSWORD` - PostgreSQL password (default: postgres)

## CI/CD

Both CI workflows now include PostgreSQL service containers:

- `.github/workflows/ci.yml` - Main CI with RSpec tests
- `.github/workflows/playwright.yml` - Playwright tests

The PostgreSQL service runs on port 5432 with:

- User: postgres
- Password: postgres
- Database: medtracker_test

## Docker Compose Files

- `docker-compose.yml` - Production deployment
- `docker-compose.dev.yml` - Local development database only
- `docker-compose.test.yml` - Test environment

## Troubleshooting

### Connection refused

Ensure PostgreSQL is running:

```bash
# Docker
docker-compose -f docker-compose.dev.yml ps

# Local PostgreSQL (macOS)
brew services list

# Local PostgreSQL (Linux)
sudo systemctl status postgresql
```

### Database does not exist

Create the databases:

```bash
rails db:create
```

### Permission denied

Check your PostgreSQL user has the correct permissions:

```bash
psql -U postgres -c "ALTER USER postgres CREATEDB;"
```
