# MedTracker Client Tools

MedTracker provides first-party Rust client tools for hosted API workflows.

- `medtracker` is a human-operated CLI.
- `medtracker-mcp` is a stdio MCP server for agent clients.

Both tools use the public `/api/v1` HTTP API. This keeps them outside the Rails
database and internal code, with no Rails shell or constant dependency.

The [API versioning policy](versioning.md) defines contract compatibility and
the migration from handwritten transport types to generated bindings.

## Native API client generation

OpenAPI Generator turns `docs/api/openapi.v1.yaml` into temporary Swift and
Kotlin packages. The packages are build inputs, not maintained SDK source. CI
generates them under `tmp/api-clients/` and compiles every generated file.
Generated reference documentation is disabled.

Generation uses the pinned OpenAPI Generator `v7.20.0` Docker image. Local
commands require Docker, Fish, and Task. Kotlin compilation requires Java 17.
Swift compilation requires Swift 6.3.3.

```text
task api-clients:generate
task api-clients:verify-generated
task api-clients:kotlin:test
task api-clients:swift:test
```

`api-clients:verify-generated` generates each package twice and compares the
results. Do not edit files under `tmp/api-clients/`; change the OpenAPI contract
or generator configuration instead.

Every non-null JSON value has one fixed type in `/api/v1`:

- Decimal values are strings, such as `"2.5"`. Nullable decimals can be `null`.
- Flexible resource identifiers are strings. Numeric identifiers use digit
  strings such as `"42"`; UUID identifiers use UUID strings.
- Numeric JSON values for these fields return the standard validation error.

To upgrade OpenAPI Generator, update the version and digest in both generation
scripts. Review both configuration files, verify deterministic output, and
compile both complete packages before merging.

## Install

Release artifacts are built for Linux x86_64, macOS x86_64, and macOS aarch64.
During development, build from the workspace:

```bash
cargo build --manifest-path client-tools/Cargo.toml --workspace
```

## Authentication

Set the hosted base URL and authenticate through the API:

```bash
medtracker --base-url https://example.invalid auth login \
  --email user@example.com \
  --password "$MEDTRACKER_PASSWORD"
```

The CLI stores non-secret profile configuration in the operating system config
directory. Access tokens are stored in the operating system keychain where
available. Tests and automation can pass `MEDTRACKER_TOKEN` to avoid touching a
developer keychain.

The tools never print token material. API errors show the server `code`,
`message`, `request_id`, and `retry_after` value when present.

## Command Groups

```bash
medtracker capabilities
medtracker auth status
medtracker households list
medtracker me --household-id HOUSEHOLD_ID
medtracker resources list --household-id HOUSEHOLD_ID --kind medications
medtracker portable export --household-id HOUSEHOLD_ID
medtracker backup export --household-id HOUSEHOLD_ID
medtracker sync snapshot --household-id HOUSEHOLD_ID
```

Use `--output json` for script-safe output. Table output is intended for
interactive use.

Commands that require unsupported backend features fail with
`unsupported_by_server` after checking `GET /api/v1/capabilities`.

## Portable Import Passphrases

Portable import commands accept passphrases through stdin only:

```bash
printf '%s' "$MEDTRACKER_IMPORT_PASSPHRASE" |
  medtracker portable import \
    --household-id HOUSEHOLD_ID \
    --file portable-export.json \
    --passphrase-stdin
```

There is no `--passphrase` argument, so passphrases are kept out of argv,
profile files, and shell history where technically possible.

## MCP

Run the stdio MCP server with:

```bash
MEDTRACKER_BASE_URL=https://example.invalid \
MEDTRACKER_TOKEN="$MEDTRACKER_TOKEN" \
medtracker-mcp
```

The server exposes typed JSON-schema tool parameters. Inspect them with:

```bash
medtracker-mcp --schema
```

The hosted `/mcp` streamable HTTP server remains available for first-party
hosted integrations. The Rust `medtracker-mcp` binary is for local stdio agent
clients and still talks to MedTracker through `/api/v1`.

## Development Gates

Run the Rust gates before pushing client-tool changes:

```bash
task client-tools:fmt
task client-tools:check
task client-tools:clippy
task client-tools:test
```

Rails gates still apply for repository changes:

```bash
task rubocop
task test
```
