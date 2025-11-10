# Docker Setup Summary

## Available Configurations

### 1. Development (`docker-compose.dev.yml`)

- **Purpose**: Local development with live code reloading
- **Database**: `medtracker_development`
- **SSL**: Disabled (development mode)
- **Volumes**: Source code mounted for live changes
- **Port**: 3000

**Usage:**

```bash
docker-compose -f docker-compose.dev.yml up
```

### 2. Production (`docker-compose.yml`)

- **Purpose**: Production-like environment for local testing
- **Database**: `medtracker_production`
- **Volumes**: Precompiled assets, production credentials key mounted
- **Port**: 3000

**Usage:**

```bash
docker-compose up
```

**Note**: Requires `config/credentials/production.key` file

### 3. Test (`docker-compose.test.yml`)

- **Purpose**: Running RSpec and Playwright tests
- **Database**: `medtracker_test`
- **Port**: 3000

## Key Files Created/Modified

1. **`bin/docker-entrypoint-web`**: Entrypoint for production Docker container
2. **`docker-compose.dev.yml`**: New development configuration
3. **`config/environments/production.rb`**: Made `force_ssl` configurable via `RAILS_FORCE_SSL` env var
4. **`Dockerfile`**: Updated asset precompilation logic

## SSL Configuration

- **Development**: No SSL (uses development mode)
- **Production (Docker)**: SSL disabled via `RAILS_FORCE_SSL=false` for local testing
- **Production (Real)**: SSL enabled by default, expects reverse proxy (nginx/caddy) handling SSL termination

## Migration Strategy (Battle-Tested Approach)

Following Nick Janetakis's docker-rails-example pattern:

1. **Migrations are NOT run automatically** in the entrypoint
2. **Run migrations explicitly** using the `./run` script:
   - First time: `./run db:setup`
   - Updates: `./run db:migrate`
3. **Entrypoint is minimal** - only copies precompiled assets to volume

### Why This Approach?

- **Explicit control**: You decide when migrations run
- **Safer deployments**: Prevents automatic schema changes on container restart
- **Better debugging**: Migration failures don't prevent container startup
- **CI/CD friendly**: Migrations can be run as separate deployment step

## Next Steps for Production Deployment

When deploying to real production with SSL:

1. Remove `RAILS_FORCE_SSL=false` from docker-compose.yml
2. Set up reverse proxy (nginx/Caddy) with Let's Encrypt
3. Configure proxy to handle SSL termination and forward to Rails on port 3000
4. Run migrations explicitly: `./run db:migrate`
