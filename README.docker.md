# Docker Setup for MedTracker

This project includes Docker Compose configurations for both production and test environments.

## Prerequisites

- Docker
- Docker Compose
- Rails master key (in `config/master.key` or as environment variable)

## Production Environment

The production environment uses PostgreSQL and runs the Rails app in production mode.

### Starting Production

```bash
# Set your Rails master key
export RAILS_MASTER_KEY=$(cat config/master.key)

# Start the services
docker-compose up -d

# View logs
docker-compose logs -f web
```

The application will be available at <http://localhost>

### Production Services

- **web**: Rails application (port 80)
- **db**: PostgreSQL database (port 5432)

### Production Commands

```bash
# Run migrations
docker-compose exec web bin/rails db:migrate

# Access Rails console
docker-compose exec web bin/rails console

# Stop services
docker-compose down

# Stop and remove volumes (WARNING: deletes data)
docker-compose down -v
```

## Test Environment

The test environment includes live reloading and is designed for running tests in isolation.

### Starting Test Environment

```bash
# Start test database and run tests
docker-compose -f docker-compose.test.yml up

# Run tests in watch mode (requires additional setup)
docker-compose -f docker-compose.test.yml run --rm test bundle exec rspec

# Run specific test file
docker-compose -f docker-compose.test.yml run --rm test bundle exec rspec spec/models/user_spec.rb
```

### Test Services

- **test**: Rails test environment with live code reloading
- **db_test**: PostgreSQL test database (port 5433)

### Test Commands

```bash
# Run tests interactively
docker-compose -f docker-compose.test.yml run --rm test bash
# Then inside container:
bundle exec rspec

# Reset test database
docker-compose -f docker-compose.test.yml run --rm test bin/rails db:test:prepare

# Stop test services
docker-compose -f docker-compose.test.yml down
```

## Live Reloading

The test environment mounts your local code directory as a volume, so changes to your code are immediately reflected in the container. This allows for rapid test-driven development:

1. Edit code locally
2. Run tests in the container
3. See results immediately

## Database Access

### Production Database

```bash
# Connect to production database
docker-compose exec db psql -U medtracker -d medtracker_production
```

### Test Database

```bash
# Connect to test database
docker-compose -f docker-compose.test.yml exec db_test psql -U medtracker_test -d medtracker_test
```

## Troubleshooting

### Database Connection Issues

If you see database connection errors:

```bash
# Check database is healthy
docker-compose ps

# View database logs
docker-compose logs db

# Restart database
docker-compose restart db
```

### Permission Issues

If you encounter permission issues with volumes:

```bash
# Fix ownership (run from host)
sudo chown -R $USER:$USER storage/ log/ tmp/
```

### Clean Slate

To start fresh:

```bash
# Stop all services and remove volumes
docker-compose down -v
docker-compose -f docker-compose.test.yml down -v

# Rebuild images
docker-compose build --no-cache
docker-compose -f docker-compose.test.yml build --no-cache
```

## Notes

- The production Dockerfile is optimized for size and security
- The test Dockerfile includes development dependencies
- Both environments use PostgreSQL 16 Alpine for efficiency
- Volumes persist data between container restarts
- The test environment uses separate ports to avoid conflicts
