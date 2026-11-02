# Docker Development

The root `compose.yaml` provides disposable PostgreSQL 18 and RustFS services for
the Loco integration and browser test suites.

Do not copy old `docker-compose` commands or run Compose services directly.
Use these current guides instead:

- [Technical quick start](docs/quick-start.md)
- [Testing](docs/testing.md)
- [Deployment and local production-image checks](docs/deployment.md)

List every supported command with:

```fish
task --list
```
