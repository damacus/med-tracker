# Rust language server MCP

The local replacement uses `isaacphi/mcp-language-server` at
`e4395849a52e18555361abab60a060802c06bf50`, with the adjacent patch applied.
The patch bounds LSP requests, serialises message writes, keeps draining stderr
after an oversized log line, and avoids eagerly opening every watched file in
rust-analyzer. Diagnostics refresh the file and use the returned full report;
request errors are returned to the caller instead of being reported as clean.
Other language servers retain their existing eager-open behaviour.

To rebuild from an upstream source checkout at that revision:

```fish
rtk git apply --unidiff-zero /path/to/med-tracker/scripts/lsp/mcp-language-server.patch
rtk proxy go test ./internal/lsp ./internal/tools ./internal/watcher/...
rtk proxy go build -o /path/to/mcp-language-server .
```

Configure the MCP command with `--workspace` set to the MedTracker checkout and
`--lsp` set to a real rust-analyzer binary. Avoid recursive mise shims. Its PATH
must include the matching Rust toolchain binaries; set `RUSTUP_TOOLCHAIN` to the
same toolchain so nested rustup commands agree. Cargo continues using the
normal user configuration and checkout target directory.

Before switching clients, check definition and references for `App`, diagnostics
for a temporary Rust file with a known syntax error, and clean diagnostics after
correcting the same file in the same server session. Remove the temporary file
before running the application build. Keep Serena enabled if any check fails.

This is a local MCP configuration change; clients must reconnect to load it.
