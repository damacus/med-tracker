# MedTracker Client Tools

MedTracker ships first-party Rust client tools for hosted API workflows.

- `medtracker` is a human-operated CLI.
- `medtracker-mcp` is a stdio MCP server for agent clients.

Both tools use the public `/api/v1` HTTP API. They do not read the Rails
database, call Rails internals, shell into Rails, or depend on Rails constants.

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
