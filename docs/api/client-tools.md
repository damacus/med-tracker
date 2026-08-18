# MedTracker Client Tools

MedTracker provides first-party Rust client tools for hosted API workflows.

- `medtracker` is a human-operated CLI.
- `medtracker-mcp` is a stdio MCP server for agent clients.

Both tools use the public `/api/v1` HTTP API. This keeps them outside the Rails
database and internal code, with no Rails shell or constant dependency.

The [API versioning policy](versioning.md) defines contract compatibility and
the migration from handwritten transport types to generated bindings.

## Generated native API clients

The checked-in Swift and Kotlin clients are generated from
`docs/api/openapi.v1.yaml`. They expose the `/api/v1` contract without adding
Rails runtime dependencies or Rails autoloaded constants.

The generated packages are kept in these directories:

- `client-tools/generated/swift` — Swift Package Manager package named
  `MedTrackerAPI`.
- `client-tools/generated/kotlin` — JVM package
  `io.medtracker.client`, published as `io.medtracker:medtracker-api-client`.

The handwritten consumer tests are kept outside generated output:

- `client-tools/swift-consumer-tests`
- `client-tools/kotlin-consumer-tests`

### Prerequisites

Install these tools before generating or testing native clients:

- Docker Engine/Desktop 24.0 or newer, to run OpenAPI Generator `v7.20.0`
  from the pinned image.
- Fish, to run the repository generation scripts.
- Task `3.52.0` or newer, to run the repository commands.
- Java 17, to compile and test the Kotlin package with its committed Gradle
  wrapper.
- Swift 6.3.3 and Swift Package Manager, to compile and test the Swift package.

### Local commands

Run these commands from the repository root:

```text
task api-clients:generate
task api-clients:verify-generated
task api-clients:verify
task api-clients:kotlin
task api-clients:kotlin:test
task api-clients:swift
task api-clients:swift:test
```

`api-clients:generate` replaces both committed packages after successful
temporary generation. `api-clients:verify-generated` performs a read-only
comparison. `api-clients:verify` also checks checksum metadata, cleanup on
generator failure, and two-generation determinism.

### Contract rules for native clients

Every non-null JSON value has one fixed type in `/api/v1`:

- Decimal values are JSON strings. Send values such as `"2.5"`, not the JSON
  number `2.5`. Nullable decimals use the same string type and may be `null`.
- Flexible resource identifiers are JSON strings. Numeric identifiers use
  digit strings such as `"42"`; UUID identifiers use UUID strings. Do not send
  JSON numbers for these fields.
- A JSON number in a decimal or flexible-identifier request field returns the
  standard validation-error envelope.
- Absence and clearing remain represented by `null` where the contract marks a
  field nullable. Optional fields may also be omitted when the contract allows
  omission.

### Regeneration and review

Never edit files below `client-tools/generated/` by hand. Change the OpenAPI
document or generator configuration, then run `task api-clients:generate`.
Review the generated diff, the `OPENAPI_SHA256` files, and the consumer tests.
Run `task api-clients:verify-generated`, both native test commands, and the
repository quality gates before committing.

To upgrade OpenAPI Generator, update the version and immutable digest in both
`client-tools/openapi-generator/generate-swift.fish` and
`client-tools/openapi-generator/generate-kotlin.fish`. Review any generator
configuration changes against the Swift 6 and Kotlin documentation, regenerate
both packages, and require two byte-for-byte identical generations. Compile
both packages and run the consumer tests before accepting the generated diff.
Record the new generator version and digest in the pull request. Do not
upgrade one language generator without regenerating and checking the other.

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
