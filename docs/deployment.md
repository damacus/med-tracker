# Deployment Guide

MedTracker includes Docker Compose configurations for easy deployment across development, production, and test environments.

## Docker Compose Overview

The project includes three Docker Compose configurations:

- `docker-compose.dev.yml` - Development with live code reloading
- `docker-compose.yml` - Production deployment
- `docker-compose.test.yml` - Test environment

## Development Deployment

### Quick Start

For development with live code reloading:

```bash
# First time setup
cp .env.example .env
docker compose -f docker-compose.dev.yml up -d
./run db:setup

# Start the app
docker compose -f docker-compose.dev.yml up
```

Access the app at `http://localhost:3000`.

### Development Commands

The `./run` script provides convenient shortcuts for common tasks:

#### Database Commands

```bash
./run db:setup      # Setup database (first time)
./run db:migrate    # Run migrations
./run db:reset      # Reset database
./run db:seed       # Load seed data
```

#### Rails Commands

```bash
./run rails console       # Open Rails console
./run rails routes        # View all routes
./run rails <command>     # Run any Rails command
```

#### Testing & Quality

```bash
./run test          # Run RSpec tests
./run format        # Auto-fix RuboCop issues
./run lint          # Check code style
```

#### Container Management

```bash
./run shell         # Open bash shell in container
./run help          # Show all available commands
```

## Production Deployment

### Prerequisites

1. Ensure you have the production credentials file:
   - `config/credentials/production.key`

2. Set up your `.env` file with production values:

```bash
RAILS_ENV=production
SECRET_KEY_BASE=your_secret_key_here
DB_HOST=postgres
DB_USERNAME=medtracker
DB_PASSWORD=secure_password
```

### Deployment Steps

1. Build and start the production containers:

```bash
docker compose up -d
```

2. Run database migrations:

```bash
./run db:migrate
```

3. Access the application at `http://localhost:3000` or your configured domain.

### Production Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `RAILS_ENV` | Rails environment (set to `production`) | Yes |
| `SECRET_KEY_BASE` | Rails secret for sessions/cookies | Yes |
| `DB_HOST` | PostgreSQL host | Yes |
| `DB_USERNAME` | PostgreSQL username | Yes |
| `DB_PASSWORD` | PostgreSQL password | Yes |
| `RAILS_SERVE_STATIC_FILES` | Serve static assets | Optional |
| `RAILS_LOG_TO_STDOUT` | Log to stdout | Optional |

### Generating Secrets

Generate a new secret key base:

```bash
docker compose run --rm web rails secret
```

Or locally:

```bash
rails secret
```

## Test Environment

Run the test suite in a containerized environment:

```bash
docker compose -f docker-compose.test.yml up --abort-on-container-exit
```

This will:
1. Build the test environment
2. Run the full test suite
3. Exit with the test suite's exit code

## Container Architecture

### Services

#### Web Service

- **Base Image**: Ruby official image
- **Purpose**: Runs the Rails application
- **Ports**: 3000
- **Dependencies**: PostgreSQL

#### PostgreSQL Service

- **Base Image**: PostgreSQL official image
- **Purpose**: Database server
- **Ports**: 5432 (not exposed in production)
- **Volumes**: Persistent data storage

### Volume Mounts

#### Development

In development, the following directories are mounted:

- `.` → `/rails` - Full application code for live reloading
- `bundle` - Persisted gems
- `node_modules` - Persisted JavaScript dependencies

#### Production

In production, the application is copied into the image at build time for better performance and security.

## Health Checks

Docker Compose includes health checks to ensure services are ready:

```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:3000/"]
  interval: 30s
  timeout: 3s
  retries: 3
```

## Scaling

Scale the web service for higher load:

```bash
docker compose up -d --scale web=3
```

**Note**: You'll need to configure a load balancer (like nginx) to distribute traffic across instances.

## Logs

View logs from running containers:

```bash
# All services
docker compose logs -f

# Specific service
docker compose logs -f web
docker compose logs -f postgres
```

## Backup and Restore

### Backup Database

```bash
docker compose exec postgres pg_dump -U postgres medtracker_production > backup.sql
```

### Restore Database

```bash
cat backup.sql | docker compose exec -T postgres psql -U postgres medtracker_production
```

## Updating

To update the application:

1. Pull the latest changes:

```bash
git pull
```

2. Rebuild containers if needed:

```bash
docker compose build
```

3. Run migrations:

```bash
./run db:migrate
```

4. Restart services:

```bash
docker compose restart web
```

## Troubleshooting

### Container Won't Start

Check logs for errors:

```bash
docker compose logs web
```

### Database Connection Issues

Verify PostgreSQL is running and healthy:

```bash
docker compose ps postgres
```

Check database connection from web container:

```bash
docker compose exec web rails db:version
```

### Port Conflicts

If port 3000 is already in use, modify the port mapping in `docker-compose.yml`:

```yaml
ports:
  - "3001:3000"  # Use port 3001 instead
```

### Permission Issues

If you encounter permission errors with mounted volumes:

```bash
docker compose run --rm web chown -R $(id -u):$(id -g) .
```

## Production Best Practices

1. **Use Environment Variables**: Never commit secrets to version control
2. **Enable HTTPS**: Use a reverse proxy (nginx, Traefik) with SSL certificates
3. **Regular Backups**: Automate database backups
4. **Monitor Logs**: Set up log aggregation and monitoring
5. **Resource Limits**: Configure memory and CPU limits in Docker Compose
6. **Health Checks**: Ensure health checks are properly configured
7. **Update Regularly**: Keep base images and dependencies up to date

## Alternative Deployment Options

### Kamal

The project includes Kamal configuration files in `.kamal/` for zero-downtime deployments. See the Kamal documentation for setup instructions.

### Heroku

MedTracker can be deployed to Heroku with minimal configuration:

```bash
heroku create medtracker-app
heroku addons:create heroku-postgresql
git push heroku main
heroku run rails db:migrate
```

### Traditional VPS

For deployment on a traditional VPS, you can:

1. Use the Docker Compose production setup
2. Or deploy using a traditional Ruby stack (Passenger, Puma, etc.)

Refer to the Rails deployment guides for your chosen stack.
