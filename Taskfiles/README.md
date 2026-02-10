# Taskfile Structure Documentation

## Overview

This project uses [Task](https://taskfile.dev) as a task runner. The Taskfiles are organized into modular components for clarity and maintainability.

## File Structure

```text
Taskfile.yml              # Main entry point - defines user-facing commands
Taskfiles/
  ├── README.md          # This file - documentation for LLMs and developers
  ├── internal.yml       # Reusable Docker Compose operations (internal use only)
  ├── dev.yml            # Development environment commands
  ├── test.yml           # Test environment commands
  └── docs.yml           # Documentation build/serve commands
```

## How It Works

### 1. Main Taskfile (Taskfile.yml)

The root `Taskfile.yml` includes the modular taskfiles and defines top-level commands:

- **includes**: Imports taskfiles under namespaces (dev:, test:, docs:, internal:)
- **tasks**: Defines user-facing commands like `task test` and `task rubocop`

### 2. Internal Tasks (Taskfiles/internal.yml)

Contains reusable Docker Compose operations marked with `internal: true`:

- **Purpose**: DRY principle - avoid repeating Docker Compose commands
- **Usage**: Called by dev.yml and test.yml with ENVIRONMENT variable
- **Key Tasks**:
  - `run`: Execute arbitrary commands in containers
  - `build`: Build Docker images
  - `up`: Start containers
  - `stop`: Stop and remove containers
  - `logs`: Follow container logs
  - `seed`: Seed database with fixtures

**Important**: These tasks require `ENVIRONMENT` variable (dev or test)

### 3. Environment-Specific Tasks

#### dev.yml - Development Environment

Commands for local development:

- `task dev:up` - Start development server
- `task dev:stop` - Stop development server
- `task dev:rebuild` - Rebuild from scratch (drops database)
- `task dev:seed` - Seed database with fixtures
- `task dev:logs` - View server logs

#### test.yml - Test Environment

Commands for running tests:

- `task test:up` - Start test server
- `task test:stop` - Stop test server
- `task test:rebuild` - Rebuild test environment (drops database)
- `task test:seed` - Seed test database
- `task test:logs` - View test logs

#### docs.yml - Documentation Environment

Commands for docs authoring:

- `task docs:serve` - Serve docs locally with live reload
- `task docs:build` - Build static docs site

## Common Usage Patterns

### Running Tests

```bash
# Run all tests
task test

# Run specific test file
task test TEST_FILE=spec/models/user_spec.rb
```

### Development Workflow

```bash
# Start development server
task dev:up

# Make code changes...

# Rebuild after migration changes
task dev:rebuild

# View logs
task dev:logs
```

### Database Operations

```bash
# Seed development database
task dev:seed

# Seed test database
task test:seed

# Reset database (via rebuild)
task test:rebuild  # For testing changes
task dev:rebuild   # For development
```

### Code Quality

```bash
# Run RuboCop
task rubocop

# Run RuboCop with autocorrect
task rubocop AUTOCORRECT=true
```

## How to Edit/Update

### Adding a New Command

1. **Determine scope**: Is it dev-only, test-only, or both?
2. **Choose file**: Add to dev.yml, test.yml, or both
3. **Use internal tasks**: Leverage existing internal tasks when possible

Example - Adding a console command:

```yaml
# In Taskfiles/dev.yml
console:
  desc: Open Rails console in development
  cmds:
    - task: internal:run
      vars:
        ENVIRONMENT: dev
        SERVICE: web
        COMMAND: 'rails console'
```

### Adding a New Internal Task

1. Add to `Taskfiles/internal.yml`
2. Mark with `internal: true`
3. Use `ENVIRONMENT` variable for flexibility
4. Document in this README

Example:

```yaml
# In Taskfiles/internal.yml
migrate:
  desc: Run database migrations
  internal: true
  vars:
    ENVIRONMENT: {ref: .ENVIRONMENT}
  cmds:
    - docker compose -f docker-compose.{{ .ENVIRONMENT }}.yml run web rails db:migrate
```

### Modifying Existing Tasks

1. **Check dependencies**: See which tasks call the one you're modifying
2. **Test both environments**: Changes to internal tasks affect dev AND test
3. **Update documentation**: Keep this README current

## Troubleshooting

### Task not found error

```bash
task: Task "dev:internal:stop" does not exist
```

**Cause**: Taskfile is trying to call an internal task that doesn't exist or has wrong namespace

**Fix**: Ensure internal tasks are marked `internal: true` and called with correct namespace

### Environment variable not passed

```bash
docker-compose..yml: no such file or directory
```

**Cause**: ENVIRONMENT variable not reaching internal task

**Fix**: Ensure vars block uses `{ref: .ENVIRONMENT}` syntax in internal.yml

### Docker Compose file not found

**Cause**: Wrong ENVIRONMENT value or missing docker-compose file

**Fix**: Verify ENVIRONMENT is "dev" or "test" and corresponding docker-compose.{env}.yml exists

## Variable Reference

### ENVIRONMENT

- **Values**: `dev` or `test`
- **Usage**: Determines which docker-compose file to use
- **Required by**: All internal tasks

### TEST_FILE

- **Values**: Path to spec file (e.g., `spec/models/user_spec.rb`)
- **Default**: `spec` (runs all tests)
- **Usage**: Specify which tests to run

### AUTOCORRECT

- **Values**: `true` or `false`
- **Default**: `false`
- **Usage**: Enable RuboCop autocorrect with `-A` flag

### NO_CACHE

- **Values**: `true` or empty
- **Default**: empty (use cache)
- **Usage**: Force Docker to rebuild without cache

## Best Practices for LLMs

1. **Always specify environment**: When testing changes, use `test` environment
2. **Use rebuild for schema changes**: Database migrations require rebuild
3. **Check task list first**: Run `task --list` to see available commands
4. **Read error messages**: They usually indicate missing variables or wrong namespace
5. **Test incrementally**: Run `task test` after making changes
6. **Use internal tasks**: Don't duplicate Docker Compose commands

## Quick Reference

| Command                    | Purpose             | Environment |
|----------------------------|---------------------|-------------|
| `task test`                | Run all tests       | test        |
| `task test TEST_FILE=path` | Run specific test   | test        |
| `task rubocop`             | Run RuboCop         | local       |
| `task dev:up`              | Start dev server    | dev         |
| `task dev:rebuild`         | Reset dev database  | dev         |
| `task test:rebuild`        | Reset test database | test        |
| `task dev:logs`            | View dev logs       | dev         |
| `task test:logs`           | View test logs      | test        |
| `task docs:serve`          | Serve docs locally  | local       |
| `task docs:build`          | Build docs site     | local       |
