# Docker Development

MedTracker uses one `compose.yaml` file and Task wrappers for development,
tests, tooling, and local production-image checks.

Do not copy old `docker-compose` commands or run Compose services directly.
Use these current guides instead:

- [Technical quick start](docs/quick-start.md)
- [Testing](docs/testing.md)
- [Deployment and local production-image checks](docs/deployment.md)

List every supported command with:

```fish
task --list
```

Use `task dev:rebuild` or `task test:rebuild` only when a destructive database
reset is intended.
