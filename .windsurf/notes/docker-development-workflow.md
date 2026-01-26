# Docker Development Workflow

## Overview

MedTracker uses Docker Compose for development with a bind mount approach that provides automatic file syncing. This document explains the development workflow for both human developers and AI assistants.

## Quick Start

```fish
# Start development environment
task dev:up

# View logs
task dev:logs

# Stop environment
task dev:stop
```

## File Syncing Strategy

### Bind Mount (Current Approach)

The development environment uses a bind mount (`.:/app`) that automatically syncs file changes between your host machine and the container.

**What gets synced automatically:**
- `app/` - Ruby code, views, components
- `config/` - Configuration files
- `lib/` - Library code
- `spec/` - Tests
- `db/` - Database migrations and seeds

**Changes take effect immediately** - Rails development mode with code reloading means your changes are reflected without restarting the server.

### When to Rebuild

Rebuild the container when you change:
- `Gemfile` or `Gemfile.lock` (new gems)
- `package.json` or `yarn.lock` (new npm packages)
- `Dockerfile` (container configuration)

```fish
task dev:build
task dev:up
```

## Docker Compose Watch (Not Recommended)

Docker Compose has a native `watch` feature, but it has limitations in our setup:

**Why we don't use it:**
1. **Bind mount conflict** - Watch mode conflicts with bind mounts, causing immediate rebuild loops
2. **Unnecessary** - The bind mount already provides instant file syncing
3. **Complexity** - Requires running services first, then watch in a separate process

**If you need explicit rebuild triggers**, manually rebuild when needed:
```fish
task dev:build && task dev:up
```

## Common Workflows

### Starting Development

```fish
# First time setup
task dev:build
task dev:up
task dev:seed

# Daily development
task dev:up
```

### Making Code Changes

1. Edit files in `app/`, `config/`, `lib/`, etc.
2. Changes are automatically synced
3. Rails reloads the code automatically
4. Refresh your browser to see changes

### Adding Dependencies

```fish
# Edit Gemfile or package.json
# Then rebuild:
task dev:build
task dev:up
```

### Database Changes

```fish
# Create migration
task internal:run ENVIRONMENT=dev COMMAND='rails g migration AddFieldToModel'

# Run migrations
task internal:run ENVIRONMENT=dev COMMAND='rails db:migrate'

# Or rebuild which runs migrations automatically
task dev:rebuild
```

### Viewing Logs

```fish
# Follow all logs
task dev:logs

# Or use docker compose directly
docker compose -f docker-compose.dev.yml logs -f web-dev
```

## For AI Assistants

### When to Use Which Command

**Use `task dev:up`** when:
- Starting the development environment
- User asks to "start the server"
- After making code changes (no rebuild needed)

**Use `task dev:build`** when:
- Gemfile or package.json changed
- Dockerfile changed
- User reports "gem not found" errors

**Use `task dev:rebuild`** when:
- Database needs to be reset
- User asks for "fresh start"
- Major structural changes

### File Change Behavior

| File Type | Action Required | Command |
|-----------|----------------|---------|
| Ruby files (app/, lib/) | None | Auto-synced |
| Config files | None | Auto-synced |
| Views/Components | None | Auto-synced |
| Migrations | Run migration | `task internal:run ENVIRONMENT=dev COMMAND='rails db:migrate'` |
| Gemfile | Rebuild | `task dev:build && task dev:up` |
| package.json | Rebuild | `task dev:build && task dev:up` |
| Dockerfile | Rebuild | `task dev:build && task dev:up` |

### Troubleshooting

**Container won't start:**
```fish
task dev:stop
task dev:build
task dev:up
```

**Database issues:**
```fish
task dev:rebuild  # Drops and recreates database
```

**Port already in use:**
```fish
task dev:stop
# Check for other processes on port 3000
lsof -ti:3000 | xargs kill -9
task dev:up
```

## Technical Details

### Bind Mount Configuration

```yaml
volumes:
  - .:/app  # Bind mount for live file syncing
  - medtracker_dev_bundle:/usr/local/bundle  # Gem cache
  - medtracker_dev_node_modules:/app/node_modules  # Node modules
  - medtracker_dev_storage:/app/storage  # Uploaded files
```

### Why Not Docker Compose Watch?

Docker Compose watch (`docker compose watch`) is designed for scenarios where:
1. You don't use bind mounts
2. You want explicit control over sync vs rebuild actions
3. You're working with compiled languages

For Rails development:
- Bind mounts provide instant syncing
- Rails' code reloading handles changes automatically
- Watch mode would add complexity without benefits

### Performance Considerations

**Bind mounts are fast on:**
- Linux (native filesystem)
- macOS with Docker Desktop's VirtioFS

**If you experience slow file syncing:**
1. Check Docker Desktop settings (enable VirtioFS)
2. Exclude large directories from sync (already configured)
3. Consider using named volumes for specific directories

## References

- [Docker Compose File Watch Documentation](https://docs.docker.com/compose/how-tos/file-watch/)
- [Rails Development Mode](https://guides.rubyonrails.org/configuring.html#rails-general-configuration)
- Project Taskfile: `Taskfile.yml`
- Development Compose: `docker-compose.dev.yml`
