# Deployment

MedTracker runs with Docker Compose in development, test, and production-style
setups.

## Compose files

- `docker-compose.dev.yml`: development stack
- `docker-compose.test.yml`: test stack
- `docker-compose.yml`: production-style stack

## Development deployment

Use Taskfile wrappers:

```bash
task dev:up
task dev:seed
```

Stop or inspect:

```bash
task dev:stop
task dev:logs
task dev:ps
```

## Test deployment

Start/stop test services when needed:

```bash
task test:up
task test:stop
task test:logs
```

Run full tests in the test environment:

```bash
task test
```

## Production-style compose run

If you need to run the production compose file locally:

```bash
docker compose -f docker-compose.yml up -d
```

Run migrations inside the web container:

```bash
docker compose -f docker-compose.yml run --rm web rails db:migrate
```

## Environment and database notes

- All environments use PostgreSQL.
- PostgreSQL version target is `18`.
- Use Rails credentials and environment variables for secrets; never commit them.

## Rebuild environments

Development rebuild (destructive to dev volumes):

```bash
task dev:rebuild
```

Test rebuild (destructive to test volumes):

```bash
task test:rebuild
```
