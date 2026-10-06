# Docker Development

The root `compose.yaml` provides PostgreSQL 18 for the Loco foundation.
The Rails rollback uses `rails/compose.yaml` and `task rails:*` wrappers for
development, tests, tooling and production-image checks. The published product
image remains Rails until the complete Loco migration is accepted.

Do not copy old `docker-compose` commands or run Compose services directly.
Use these current guides instead:

- [Technical quick start](docs/quick-start.md)
- [Testing](docs/testing.md)
- [Deployment and local production-image checks](docs/deployment.md)

List every supported command with:

```fish
task --list
```

Use `task rails:dev:rebuild` or `task rails:test:rebuild` only when a destructive database
reset is intended.
