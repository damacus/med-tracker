# Rust CLI And MCP Tooling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build first-party Rust client tooling for MedTracker: a human-operated CLI and an MCP server that both talk to the hosted API over HTTP only.

**Architecture:** Create a Rust workspace under `client-tools/` with one shared API client crate and two binaries: `medtracker` and `medtracker-mcp`. The CLI and MCP server must not call Rails internals, read the Rails database, shell out to Rails, or rely on Rails constants; every operation goes through `/api/v1` using bearer sessions or app tokens. The implementation should start with read-only capability/auth/profile/resource commands, then add portable import/export, sync, backups, and admin tools as backend endpoints mature.

**Tech Stack:** Rust stable, Cargo workspace, `clap` derive for CLI parsing, `tokio`, `reqwest` with JSON and rustls TLS, `serde`, `serde_json`, `thiserror`, `directories`, `keyring`, `comfy-table`, `tracing`, `rmcp` for MCP server tools, `wiremock`, `assert_cmd`, `predicates`, `tempfile`, `insta`, existing Rails `/api/v1` OpenAPI contract.

---

## Scope

This plan covers issue `#1490`: Rust CLI and MCP tooling. It intentionally does not add a Ruby Thor CLI. It also does not expand backend API features by itself; when a command needs an API endpoint that does not exist yet, the command must return a clear `unsupported_by_server` error until the backend issue lands.

The CLI and MCP server are one client-tools product with shared crates:

- The shared API client crate owns auth headers, request IDs, typed errors, JSON decoding, retries, and generated or hand-maintained API models.
- The CLI binary owns command parsing, terminal output, credential storage, file I/O, and exit codes.
- The MCP binary owns MCP tool definitions and maps tool calls to shared API client calls.

## Current API Inputs

The current Rails branch exposes:

- `GET /api/v1/capabilities`
- `POST /api/v1/auth/login`
- `POST /api/v1/auth/refresh`
- `DELETE /api/v1/auth/logout`
- Household-scoped profile and core resources under `/api/v1/households/:household_id`
- Portable export/import v1 endpoints
- Checked-in OpenAPI baseline at `docs/api/openapi.v1.yaml`

The current capabilities document marks CLI and MCP as:

```json
{
  "client_tools": {
    "cli": { "supported": false, "status": "deferred" },
    "mcp_server": { "supported": false, "status": "deferred" },
    "diagnostics": ["request_id", "retry_after"]
  }
}
```

The final client-tool release must update this only when the tools are installable and covered by tests.

## File Structure

Create this structure:

```text
client-tools/
  Cargo.toml
  rust-toolchain.toml
  README.md
  crates/
    medtracker-api-client/
      Cargo.toml
      src/
        lib.rs
        auth.rs
        capabilities.rs
        client.rs
        error.rs
        models.rs
        portable.rs
        resources.rs
    medtracker-cli/
      Cargo.toml
      src/
        main.rs
        args.rs
        commands/
          mod.rs
          auth.rs
          backup.rs
          households.rs
          me.rs
          portable.rs
          resources.rs
          sync.rs
        config.rs
        output.rs
        secrets.rs
    medtracker-mcp/
      Cargo.toml
      src/
        main.rs
        server.rs
        tools/
          mod.rs
          capabilities.rs
          households.rs
          portable.rs
          resources.rs
  tests/
    fixtures/
      capabilities.json
      me.json
      people.json
      portable-export.json
```

Modify these Rails/repo files:

```text
Taskfile.yml
docs/api/openapi.v1.yaml
app/controllers/api/v1/capabilities_controller.rb
spec/requests/api/v1/capabilities_spec.rb
.github/workflows/ci.yml
renovate.json
README.md
docs/index.md
docs/api/client-tools.md
```

Responsibilities:

- `client-tools/crates/medtracker-api-client`: reusable HTTP-only client for both binaries.
- `client-tools/crates/medtracker-cli`: CLI commands, terminal output, local credential storage.
- `client-tools/crates/medtracker-mcp`: MCP server tools and schema definitions.
- `docs/api/client-tools.md`: user and operator documentation.
- `Taskfile.yml`: project-standard commands for testing and formatting Rust tools.
- `.github/workflows/ci.yml`: CI gates for the Rust workspace.

## Task 1: Rust Workspace Skeleton

**Files:**
- Create: `client-tools/Cargo.toml`
- Create: `client-tools/rust-toolchain.toml`
- Create: `client-tools/crates/medtracker-api-client/Cargo.toml`
- Create: `client-tools/crates/medtracker-cli/Cargo.toml`
- Create: `client-tools/crates/medtracker-mcp/Cargo.toml`
- Create: `client-tools/crates/medtracker-api-client/src/lib.rs`
- Create: `client-tools/crates/medtracker-cli/src/main.rs`
- Create: `client-tools/crates/medtracker-mcp/src/main.rs`
- Modify: `Taskfile.yml`

- [ ] **Step 1: Write the failing workspace smoke test**

Run:

```fish
cargo metadata --manifest-path client-tools/Cargo.toml --no-deps
```

Expected: FAIL with an error that `client-tools/Cargo.toml` does not exist.

- [ ] **Step 2: Create the workspace manifest**

Create `client-tools/Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
  "crates/medtracker-api-client",
  "crates/medtracker-cli",
  "crates/medtracker-mcp",
]

[workspace.package]
edition = "2024"
license = "Apache-2.0"
repository = "https://github.com/damacus/med-tracker"

[workspace.dependencies]
anyhow = "1"
assert_cmd = "2"
clap = { version = "4", features = ["derive", "env"] }
comfy-table = "7"
directories = "6"
insta = { version = "1", features = ["json"] }
keyring = "3"
predicates = "3"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
rmcp = { version = "0.6", features = ["server", "macros", "transport-io"] }
schemars = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tempfile = "3"
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "fs"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
url = "2"
wiremock = "0.6"
```

- [ ] **Step 3: Create Rust toolchain file**

Create `client-tools/rust-toolchain.toml`:

```toml
[toolchain]
channel = "stable"
components = ["clippy", "rustfmt"]
```

- [ ] **Step 4: Create API client crate manifest**

Create `client-tools/crates/medtracker-api-client/Cargo.toml`:

```toml
[package]
name = "medtracker-api-client"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
reqwest.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tokio.workspace = true
url.workspace = true

[dev-dependencies]
wiremock.workspace = true
```

- [ ] **Step 5: Create CLI crate manifest**

Create `client-tools/crates/medtracker-cli/Cargo.toml`:

```toml
[package]
name = "medtracker-cli"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true

[[bin]]
name = "medtracker"
path = "src/main.rs"

[dependencies]
anyhow.workspace = true
clap.workspace = true
comfy-table.workspace = true
directories.workspace = true
keyring.workspace = true
medtracker-api-client = { path = "../medtracker-api-client" }
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true

[dev-dependencies]
assert_cmd.workspace = true
predicates.workspace = true
tempfile.workspace = true
wiremock.workspace = true
```

- [ ] **Step 6: Create MCP crate manifest**

Create `client-tools/crates/medtracker-mcp/Cargo.toml`:

```toml
[package]
name = "medtracker-mcp"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true

[[bin]]
name = "medtracker-mcp"
path = "src/main.rs"

[dependencies]
anyhow.workspace = true
medtracker-api-client = { path = "../medtracker-api-client" }
rmcp.workspace = true
schemars.workspace = true
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true

[dev-dependencies]
wiremock.workspace = true
```

- [ ] **Step 7: Add minimal crate entrypoints**

Create `client-tools/crates/medtracker-api-client/src/lib.rs`:

```rust
pub const USER_AGENT: &str = concat!("medtracker-client-tools/", env!("CARGO_PKG_VERSION"));
```

Create `client-tools/crates/medtracker-cli/src/main.rs`:

```rust
fn main() {
    println!("medtracker CLI is installed");
}
```

Create `client-tools/crates/medtracker-mcp/src/main.rs`:

```rust
fn main() {
    println!("medtracker MCP server is installed");
}
```

- [ ] **Step 8: Add project task wrappers**

Modify `Taskfile.yml` under `tasks:`:

```yaml
  client-tools:fmt:
    desc: Format Rust client tools
    cmds:
      - cargo fmt --manifest-path client-tools/Cargo.toml --all

  client-tools:check:
    desc: Check Rust client tools
    cmds:
      - cargo check --manifest-path client-tools/Cargo.toml --workspace --all-targets

  client-tools:clippy:
    desc: Lint Rust client tools
    cmds:
      - cargo clippy --manifest-path client-tools/Cargo.toml --workspace --all-targets -- -D warnings

  client-tools:test:
    desc: Test Rust client tools
    cmds:
      - cargo test --manifest-path client-tools/Cargo.toml --workspace
```

- [ ] **Step 9: Run workspace checks**

Run:

```fish
task client-tools:fmt
task client-tools:check
task client-tools:clippy
task client-tools:test
```

Expected: PASS.

- [ ] **Step 10: Commit**

```fish
git add client-tools Taskfile.yml
git commit -m "chore(client-tools): add rust workspace"
```

## Task 2: Shared API Client Foundation

**Files:**
- Create: `client-tools/crates/medtracker-api-client/src/client.rs`
- Create: `client-tools/crates/medtracker-api-client/src/error.rs`
- Create: `client-tools/crates/medtracker-api-client/src/models.rs`
- Modify: `client-tools/crates/medtracker-api-client/src/lib.rs`
- Test: `client-tools/crates/medtracker-api-client/src/client.rs`

- [ ] **Step 1: Write failing tests for request construction and error decoding**

Append this test module to `client-tools/crates/medtracker-api-client/src/client.rs` while creating the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn get_json_sends_bearer_and_decodes_data() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/capabilities"))
            .and(header("authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": { "format": "medtracker.api.capabilities.v1" }
            })))
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri(), Some("test-token".to_string())).unwrap();
        let value: serde_json::Value = client.get_json("/api/v1/capabilities").await.unwrap();

        assert_eq!(value["data"]["format"], "medtracker.api.capabilities.v1");
    }

    #[tokio::test]
    async fn get_json_decodes_api_error_with_request_id() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/households/1/me"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": {
                    "code": "unauthorized",
                    "message": "Authentication required",
                    "request_id": "req-test"
                }
            })))
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri(), None).unwrap();
        let error = client
            .get_json::<serde_json::Value>("/api/v1/households/1/me")
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            ApiError::Server {
                status: 401,
                ref code,
                ref request_id,
                ..
            } if code == "unauthorized" && request_id.as_deref() == Some("req-test")
        ));
    }
}
```

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-api-client get_json
```

Expected: FAIL because `ApiClient` and `ApiError` are not defined.

- [ ] **Step 2: Implement typed error envelope**

Create `client-tools/crates/medtracker-api-client/src/error.rs`:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

#[derive(Debug, Deserialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    pub request_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),
    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("{code}: {message}")]
    Server {
        status: u16,
        code: String,
        message: String,
        request_id: Option<String>,
    },
}
```

- [ ] **Step 3: Implement reusable reqwest client**

Create `client-tools/crates/medtracker-api-client/src/client.rs`:

```rust
use std::time::Duration;

use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use url::Url;

use crate::error::{ApiError, ErrorEnvelope};
use crate::USER_AGENT;

#[derive(Clone)]
pub struct ApiClient {
    base_url: Url,
    bearer_token: Option<String>,
    http: reqwest::Client,
}

impl ApiClient {
    pub fn new(base_url: impl AsRef<str>, bearer_token: Option<String>) -> Result<Self, ApiError> {
        let mut parsed = Url::parse(base_url.as_ref())?;
        if !parsed.path().ends_with('/') {
            parsed.set_path(&format!("{}/", parsed.path().trim_end_matches('/')));
        }

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(5))
            .user_agent(USER_AGENT)
            .build()?;

        Ok(Self {
            base_url: parsed,
            bearer_token,
            http,
        })
    }

    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let response = self
            .request(reqwest::Method::GET, path)
            .send()
            .await?;

        self.decode(response).await
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = self.base_url.join(path.trim_start_matches('/')).expect("valid API path");
        let request = self.http.request(method, url);

        match self.bearer_token.as_deref() {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    async fn decode<T: DeserializeOwned>(&self, response: reqwest::Response) -> Result<T, ApiError> {
        let status = response.status();
        if status.is_success() {
            return Ok(response.json::<T>().await?);
        }

        Err(server_error(status, response).await)
    }
}

async fn server_error(status: StatusCode, response: reqwest::Response) -> ApiError {
    match response.json::<ErrorEnvelope>().await {
        Ok(envelope) => ApiError::Server {
            status: status.as_u16(),
            code: envelope.error.code,
            message: envelope.error.message,
            request_id: envelope.error.request_id,
        },
        Err(error) => ApiError::Http(error),
    }
}
```

- [ ] **Step 4: Wire modules**

Modify `client-tools/crates/medtracker-api-client/src/lib.rs`:

```rust
pub mod client;
pub mod error;
pub mod models;

pub use client::ApiClient;
pub use error::ApiError;

pub const USER_AGENT: &str = concat!("medtracker-client-tools/", env!("CARGO_PKG_VERSION"));
```

Create `client-tools/crates/medtracker-api-client/src/models.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct DataEnvelope<T> {
    pub data: T,
}
```

- [ ] **Step 5: Run tests**

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-api-client get_json
task client-tools:clippy
```

Expected: PASS.

- [ ] **Step 6: Commit**

```fish
git add client-tools
git commit -m "feat(client-tools): add shared api client"
```

## Task 3: Capabilities And Server Compatibility

**Files:**
- Create: `client-tools/crates/medtracker-api-client/src/capabilities.rs`
- Modify: `client-tools/crates/medtracker-api-client/src/lib.rs`
- Modify: `client-tools/crates/medtracker-api-client/src/client.rs`
- Test: `client-tools/crates/medtracker-api-client/src/capabilities.rs`

- [ ] **Step 1: Write failing capabilities tests**

Create `client-tools/crates/medtracker-api-client/src/capabilities.rs`:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Capabilities {
    pub format: String,
    pub api_version: String,
}

#[cfg(test)]
mod tests {
    use crate::client::ApiClient;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn capabilities_reads_public_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/capabilities"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "format": "medtracker.api.capabilities.v1",
                    "api_version": "v1"
                }
            })))
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri(), None).unwrap();
        let capabilities = client.capabilities().await.unwrap();

        assert_eq!(capabilities.format, "medtracker.api.capabilities.v1");
        assert_eq!(capabilities.api_version, "v1");
    }
}
```

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-api-client capabilities_reads_public_endpoint
```

Expected: FAIL because `ApiClient::capabilities` is not defined.

- [ ] **Step 2: Implement capabilities call**

Modify `client-tools/crates/medtracker-api-client/src/lib.rs`:

```rust
pub mod capabilities;
pub mod client;
pub mod error;
pub mod models;

pub use capabilities::Capabilities;
pub use client::ApiClient;
pub use error::ApiError;

pub const USER_AGENT: &str = concat!("medtracker-client-tools/", env!("CARGO_PKG_VERSION"));
```

Modify `client-tools/crates/medtracker-api-client/src/client.rs`:

```rust
use std::time::Duration;

use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use url::Url;

use crate::capabilities::Capabilities;
use crate::error::{ApiError, ErrorEnvelope};
use crate::models::DataEnvelope;
use crate::USER_AGENT;

#[derive(Clone)]
pub struct ApiClient {
    base_url: Url,
    bearer_token: Option<String>,
    http: reqwest::Client,
}

impl ApiClient {
    pub fn new(base_url: impl AsRef<str>, bearer_token: Option<String>) -> Result<Self, ApiError> {
        let mut parsed = Url::parse(base_url.as_ref())?;
        if !parsed.path().ends_with('/') {
            parsed.set_path(&format!("{}/", parsed.path().trim_end_matches('/')));
        }

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(5))
            .user_agent(USER_AGENT)
            .build()?;

        Ok(Self {
            base_url: parsed,
            bearer_token,
            http,
        })
    }

    pub async fn capabilities(&self) -> Result<Capabilities, ApiError> {
        let envelope: DataEnvelope<Capabilities> = self.get_json("/api/v1/capabilities").await?;
        Ok(envelope.data)
    }

    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let response = self
            .request(reqwest::Method::GET, path)
            .send()
            .await?;

        self.decode(response).await
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = self.base_url.join(path.trim_start_matches('/')).expect("valid API path");
        let request = self.http.request(method, url);

        match self.bearer_token.as_deref() {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    async fn decode<T: DeserializeOwned>(&self, response: reqwest::Response) -> Result<T, ApiError> {
        let status = response.status();
        if status.is_success() {
            return Ok(response.json::<T>().await?);
        }

        Err(server_error(status, response).await)
    }
}

async fn server_error(status: StatusCode, response: reqwest::Response) -> ApiError {
    match response.json::<ErrorEnvelope>().await {
        Ok(envelope) => ApiError::Server {
            status: status.as_u16(),
            code: envelope.error.code,
            message: envelope.error.message,
            request_id: envelope.error.request_id,
        },
        Err(error) => ApiError::Http(error),
    }
}
```

- [ ] **Step 3: Run tests**

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-api-client capabilities
task client-tools:clippy
```

Expected: PASS.

- [ ] **Step 4: Commit**

```fish
git add client-tools
git commit -m "feat(client-tools): read api capabilities"
```

## Task 4: CLI Argument Model And Output Modes

**Files:**
- Create: `client-tools/crates/medtracker-cli/src/args.rs`
- Create: `client-tools/crates/medtracker-cli/src/output.rs`
- Modify: `client-tools/crates/medtracker-cli/src/main.rs`
- Test: `client-tools/crates/medtracker-cli/tests/cli_help.rs`

- [ ] **Step 1: Write failing CLI help tests**

Create `client-tools/crates/medtracker-cli/tests/cli_help.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn help_lists_top_level_commands() {
    let mut cmd = Command::cargo_bin("medtracker").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(contains("auth"))
        .stdout(contains("capabilities"))
        .stdout(contains("households"))
        .stdout(contains("portable"))
        .stdout(contains("sync"));
}

#[test]
fn output_accepts_json_and_table() {
    let mut cmd = Command::cargo_bin("medtracker").unwrap();
    cmd.args(["--output", "json", "--help"])
        .assert()
        .success()
        .stdout(contains("--output"));
}
```

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-cli cli_help
```

Expected: FAIL because the binary only prints a static line.

- [ ] **Step 2: Implement clap arguments**

Create `client-tools/crates/medtracker-cli/src/args.rs`:

```rust
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "medtracker")]
#[command(version, about = "MedTracker API client")]
pub struct Cli {
    #[arg(long, env = "MEDTRACKER_API_URL", default_value = "http://localhost:3000")]
    pub api_url: String,

    #[arg(long, value_enum, default_value_t = OutputMode::Table)]
    pub output: OutputMode,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputMode {
    Json,
    Table,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Capabilities,
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    Households {
        #[command(subcommand)]
        command: HouseholdCommand,
    },
    Me,
    Portable {
        #[command(subcommand)]
        command: PortableCommand,
    },
    Sync {
        #[command(subcommand)]
        command: SyncCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    Login { email: String },
    Logout,
    Status,
}

#[derive(Debug, Subcommand)]
pub enum HouseholdCommand {
    List,
    Use { household_id: i64 },
}

#[derive(Debug, Subcommand)]
pub enum PortableCommand {
    Export { path: String },
    Import { path: String, #[arg(long)] dry_run: bool },
}

#[derive(Debug, Subcommand)]
pub enum SyncCommand {
    Snapshot,
    Changes,
}
```

Create `client-tools/crates/medtracker-cli/src/output.rs`:

```rust
use anyhow::Result;
use comfy_table::Table;
use serde::Serialize;

use crate::args::OutputMode;

pub fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub fn print_key_values(mode: OutputMode, rows: &[(&str, &str)]) -> Result<()> {
    match mode {
        OutputMode::Json => {
            let value = rows
                .iter()
                .map(|(key, value)| (*key, *value))
                .collect::<std::collections::BTreeMap<_, _>>();
            print_json(&value)
        }
        OutputMode::Table => {
            let mut table = Table::new();
            table.set_header(vec!["Field", "Value"]);
            for (key, value) in rows {
                table.add_row(vec![*key, *value]);
            }
            println!("{table}");
            Ok(())
        }
    }
}
```

Modify `client-tools/crates/medtracker-cli/src/main.rs`:

```rust
mod args;
mod output;

use anyhow::Result;
use clap::Parser;

use args::{Cli, Command};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Capabilities => output::print_key_values(
            cli.output,
            &[("status", "not-connected"), ("api_url", cli.api_url.as_str())],
        )?,
        Command::Auth { .. }
        | Command::Households { .. }
        | Command::Me
        | Command::Portable { .. }
        | Command::Sync { .. } => {
            output::print_key_values(cli.output, &[("status", "command-not-wired")])?;
        }
    }

    Ok(())
}
```

- [ ] **Step 3: Run CLI tests**

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-cli cli_help
task client-tools:clippy
```

Expected: PASS.

- [ ] **Step 4: Commit**

```fish
git add client-tools
git commit -m "feat(client-tools): define cli command surface"
```

## Task 5: CLI Config And Credential Storage

**Files:**
- Create: `client-tools/crates/medtracker-cli/src/config.rs`
- Create: `client-tools/crates/medtracker-cli/src/secrets.rs`
- Modify: `client-tools/crates/medtracker-cli/src/main.rs`
- Test: `client-tools/crates/medtracker-cli/src/config.rs`
- Test: `client-tools/crates/medtracker-cli/src/secrets.rs`

- [ ] **Step 1: Write failing config tests**

Create `client-tools/crates/medtracker-cli/src/config.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ProfileConfig {
    pub api_url: String,
    pub household_id: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn config_round_trips_profile() {
        let dir = TempDir::new().unwrap();
        let store = ConfigStore::new_for_dir(dir.path().to_path_buf());
        let profile = ProfileConfig {
            api_url: "https://medtracker.example.test".to_string(),
            household_id: Some(42),
        };

        store.save("default", &profile).await.unwrap();
        let loaded = store.load("default").await.unwrap();

        assert_eq!(loaded.api_url, profile.api_url);
        assert_eq!(loaded.household_id, Some(42));
    }
}
```

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-cli config_round_trips_profile
```

Expected: FAIL because `ConfigStore` is not defined.

- [ ] **Step 2: Implement file-backed config**

Replace `client-tools/crates/medtracker-cli/src/config.rs` with:

```rust
use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ProfileConfig {
    pub api_url: String,
    pub household_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    root: PathBuf,
}

impl ConfigStore {
    pub fn new() -> Result<Self> {
        let dirs = ProjectDirs::from("com", "MedTracker", "MedTracker")
            .context("could not resolve user config directory")?;
        Ok(Self {
            root: dirs.config_dir().to_path_buf(),
        })
    }

    pub fn new_for_dir(root: PathBuf) -> Self {
        Self { root }
    }

    pub async fn save(&self, name: &str, profile: &ProfileConfig) -> Result<()> {
        fs::create_dir_all(&self.root).await?;
        let path = self.profile_path(name);
        let body = serde_json::to_vec_pretty(profile)?;
        fs::write(path, body).await?;
        Ok(())
    }

    pub async fn load(&self, name: &str) -> Result<ProfileConfig> {
        let body = fs::read(self.profile_path(name)).await?;
        Ok(serde_json::from_slice(&body)?)
    }

    fn profile_path(&self, name: &str) -> PathBuf {
        self.root.join(format!("{name}.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn config_round_trips_profile() {
        let dir = TempDir::new().unwrap();
        let store = ConfigStore::new_for_dir(dir.path().to_path_buf());
        let profile = ProfileConfig {
            api_url: "https://medtracker.example.test".to_string(),
            household_id: Some(42),
        };

        store.save("default", &profile).await.unwrap();
        let loaded = store.load("default").await.unwrap();

        assert_eq!(loaded.api_url, profile.api_url);
        assert_eq!(loaded.household_id, Some(42));
    }
}
```

Modify `client-tools/crates/medtracker-cli/src/main.rs`:

```rust
mod args;
mod config;
mod output;

use anyhow::Result;
use clap::Parser;

use args::{Cli, Command};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Capabilities => output::print_key_values(
            cli.output,
            &[("status", "not-connected"), ("api_url", cli.api_url.as_str())],
        )?,
        Command::Auth { .. }
        | Command::Households { .. }
        | Command::Me
        | Command::Portable { .. }
        | Command::Sync { .. } => {
            output::print_key_values(cli.output, &[("status", "command-not-wired")])?;
        }
    }

    Ok(())
}
```

- [ ] **Step 3: Add a keyring-backed secret store**

Create `client-tools/crates/medtracker-cli/src/secrets.rs`:

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};

pub trait SecretStore: Send + Sync {
    fn set_token(&self, profile: &str, token_name: &str, value: &str) -> Result<()>;
    fn get_token(&self, profile: &str, token_name: &str) -> Result<Option<String>>;
    fn delete_token(&self, profile: &str, token_name: &str) -> Result<()>;
}

#[derive(Debug, Default, Clone)]
pub struct MemorySecretStore {
    values: Arc<Mutex<HashMap<String, String>>>,
}

impl MemorySecretStore {
    fn key(profile: &str, token_name: &str) -> String {
        format!("{profile}:{token_name}")
    }
}

impl SecretStore for MemorySecretStore {
    fn set_token(&self, profile: &str, token_name: &str, value: &str) -> Result<()> {
        self.values
            .lock()
            .expect("memory secret store lock poisoned")
            .insert(Self::key(profile, token_name), value.to_string());
        Ok(())
    }

    fn get_token(&self, profile: &str, token_name: &str) -> Result<Option<String>> {
        Ok(self
            .values
            .lock()
            .expect("memory secret store lock poisoned")
            .get(&Self::key(profile, token_name))
            .cloned())
    }

    fn delete_token(&self, profile: &str, token_name: &str) -> Result<()> {
        self.values
            .lock()
            .expect("memory secret store lock poisoned")
            .remove(&Self::key(profile, token_name));
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct KeyringSecretStore;

impl KeyringSecretStore {
    fn entry(profile: &str, token_name: &str) -> Result<keyring::Entry> {
        keyring::Entry::new("medtracker", &format!("{profile}:{token_name}"))
            .context("could not open OS keyring entry")
    }
}

impl SecretStore for KeyringSecretStore {
    fn set_token(&self, profile: &str, token_name: &str, value: &str) -> Result<()> {
        Self::entry(profile, token_name)?
            .set_password(value)
            .context("could not save token in OS keyring")
    }

    fn get_token(&self, profile: &str, token_name: &str) -> Result<Option<String>> {
        match Self::entry(profile, token_name)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error).context("could not read token from OS keyring"),
        }
    }

    fn delete_token(&self, profile: &str, token_name: &str) -> Result<()> {
        match Self::entry(profile, token_name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error).context("could not delete token from OS keyring"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_secret_store_round_trips_without_keychain_access() {
        let store = MemorySecretStore::default();

        store.set_token("default", "access", "secret-token").unwrap();
        assert_eq!(
            store.get_token("default", "access").unwrap(),
            Some("secret-token".to_string())
        );

        store.delete_token("default", "access").unwrap();
        assert_eq!(store.get_token("default", "access").unwrap(), None);
    }
}
```

Modify `client-tools/crates/medtracker-cli/src/main.rs`:

```rust
mod args;
mod config;
mod output;
mod secrets;
```

The OS-backed keyring implementation must be used by real commands. `MemorySecretStore` is test-only support for unit and command tests that must not touch the developer's login keychain.

- [ ] **Step 4: Run config and secret-store tests**

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-cli config_round_trips_profile
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-cli memory_secret_store_round_trips_without_keychain_access
task client-tools:clippy
```

Expected: PASS.

- [ ] **Step 5: Commit**

```fish
git add client-tools
git commit -m "feat(client-tools): persist cli profiles"
```

## Task 6: Auth Login, Refresh, Logout

**Files:**
- Create: `client-tools/crates/medtracker-api-client/src/auth.rs`
- Modify: `client-tools/crates/medtracker-api-client/src/client.rs`
- Modify: `client-tools/crates/medtracker-api-client/src/lib.rs`
- Create: `client-tools/crates/medtracker-cli/src/commands/auth.rs`
- Create: `client-tools/crates/medtracker-cli/src/commands/mod.rs`
- Modify: `client-tools/crates/medtracker-cli/src/main.rs`
- Test: `client-tools/crates/medtracker-api-client/src/auth.rs`
- Test: `client-tools/crates/medtracker-cli/tests/auth.rs`

- [ ] **Step 1: Write failing API auth tests**

Create `client-tools/crates/medtracker-api-client/src/auth.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub device_name: String,
    pub household_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub household: Option<HouseholdSummary>,
}

#[derive(Debug, Deserialize)]
pub struct HouseholdSummary {
    pub id: i64,
    pub slug: String,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ApiClient;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn login_posts_password_payload() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/auth/login"))
            .and(body_json(serde_json::json!({
                "email": "user@example.test",
                "password": "secret",
                "device_name": "medtracker-cli",
                "household_id": null
            })))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "data": {
                    "access_token": "access",
                    "refresh_token": "refresh",
                    "household": { "id": 1, "slug": "home", "name": "Home" }
                }
            })))
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri(), None).unwrap();
        let response = client
            .login(LoginRequest {
                email: "user@example.test".to_string(),
                password: "secret".to_string(),
                device_name: "medtracker-cli".to_string(),
                household_id: None,
            })
            .await
            .unwrap();

        assert_eq!(response.access_token, "access");
        assert_eq!(response.refresh_token, "refresh");
        assert_eq!(response.household.unwrap().id, 1);
    }
}
```

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-api-client login_posts_password_payload
```

Expected: FAIL because `ApiClient::login` is not defined.

- [ ] **Step 2: Implement POST JSON support and login**

Modify `client-tools/crates/medtracker-api-client/src/client.rs` by adding imports and methods:

```rust
use serde::Serialize;

use crate::auth::{LoginRequest, LoginResponse};
```

Add inside `impl ApiClient`:

```rust
    pub async fn login(&self, request: LoginRequest) -> Result<LoginResponse, ApiError> {
        let envelope: DataEnvelope<LoginResponse> = self.post_json("/api/v1/auth/login", &request).await?;
        Ok(envelope.data)
    }

    pub async fn post_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let response = self
            .request(reqwest::Method::POST, path)
            .json(body)
            .send()
            .await?;

        self.decode(response).await
    }
```

Modify `client-tools/crates/medtracker-api-client/src/lib.rs`:

```rust
pub mod auth;
pub mod capabilities;
pub mod client;
pub mod error;
pub mod models;

pub use auth::{HouseholdSummary, LoginRequest, LoginResponse};
pub use capabilities::Capabilities;
pub use client::ApiClient;
pub use error::ApiError;

pub const USER_AGENT: &str = concat!("medtracker-client-tools/", env!("CARGO_PKG_VERSION"));
```

- [ ] **Step 3: Run API auth tests**

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-api-client login_posts_password_payload
```

Expected: PASS.

- [ ] **Step 4: Add CLI auth command tests**

Create `client-tools/crates/medtracker-cli/tests/auth.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn auth_status_reports_missing_profile() {
    let mut cmd = Command::cargo_bin("medtracker").unwrap();
    cmd.args(["auth", "status"])
        .env("MEDTRACKER_CONFIG_DIR", "/tmp/medtracker-test-missing")
        .assert()
        .success()
        .stdout(contains("command-not-wired"));
}
```

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-cli auth_status_reports_missing_profile
```

Expected: PASS while auth remains unwired. This preserves a checkpoint before replacing `command-not-wired`.

- [ ] **Step 5: Implement real auth command module**

Create `client-tools/crates/medtracker-cli/src/commands/mod.rs`:

```rust
pub mod auth;
```

Create `client-tools/crates/medtracker-cli/src/commands/auth.rs`:

```rust
use anyhow::Result;

use crate::args::AuthCommand;
use crate::output;

pub async fn run(command: AuthCommand, output_mode: crate::args::OutputMode) -> Result<()> {
    match command {
        AuthCommand::Login { email } => output::print_key_values(
            output_mode,
            &[("auth", "login"), ("email", email.as_str())],
        )?,
        AuthCommand::Logout => output::print_key_values(output_mode, &[("auth", "logout")])?,
        AuthCommand::Status => output::print_key_values(output_mode, &[("auth", "status")])?,
    }

    Ok(())
}
```

Modify `client-tools/crates/medtracker-cli/src/main.rs`:

```rust
mod args;
mod commands;
mod config;
mod output;

use anyhow::Result;
use clap::Parser;

use args::{Cli, Command};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Capabilities => output::print_key_values(
            cli.output,
            &[("status", "not-connected"), ("api_url", cli.api_url.as_str())],
        )?,
        Command::Auth { command } => commands::auth::run(command, cli.output).await?,
        Command::Households { .. }
        | Command::Me
        | Command::Portable { .. }
        | Command::Sync { .. } => {
            output::print_key_values(cli.output, &[("status", "command-not-wired")])?;
        }
    }

    Ok(())
}
```

- [ ] **Step 6: Run auth command tests**

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-cli auth_status_reports_missing_profile
task client-tools:clippy
```

Expected: PASS.

- [ ] **Step 7: Commit**

```fish
git add client-tools
git commit -m "feat(client-tools): add auth command foundation"
```

## Task 7: Read-Only Resource Commands

**Files:**
- Create: `client-tools/crates/medtracker-api-client/src/resources.rs`
- Modify: `client-tools/crates/medtracker-api-client/src/lib.rs`
- Create: `client-tools/crates/medtracker-cli/src/commands/resources.rs`
- Modify: `client-tools/crates/medtracker-cli/src/commands/mod.rs`
- Modify: `client-tools/crates/medtracker-cli/src/args.rs`
- Test: `client-tools/crates/medtracker-api-client/src/resources.rs`
- Test: `client-tools/crates/medtracker-cli/tests/resources.rs`

- [ ] **Step 1: Write failing API resource tests**

Create `client-tools/crates/medtracker-api-client/src/resources.rs`:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Person {
    pub id: i64,
    pub portable_id: Option<String>,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use crate::ApiClient;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn list_people_reads_household_collection() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/households/1/people"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    { "id": 1, "portable_id": "person-1", "name": "Jane" }
                ],
                "meta": { "page": 1, "per_page": 20, "total_count": 1 }
            })))
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri(), Some("token".to_string())).unwrap();
        let people = client.list_people(1).await.unwrap();

        assert_eq!(people.len(), 1);
        assert_eq!(people[0].name, "Jane");
    }
}
```

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-api-client list_people_reads_household_collection
```

Expected: FAIL because `ApiClient::list_people` is not defined.

- [ ] **Step 2: Implement people collection call**

Modify `client-tools/crates/medtracker-api-client/src/resources.rs`:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CollectionEnvelope<T> {
    pub data: Vec<T>,
}

#[derive(Debug, Deserialize)]
pub struct Person {
    pub id: i64,
    pub portable_id: Option<String>,
    pub name: String,
}
```

Modify `client-tools/crates/medtracker-api-client/src/client.rs` by adding:

```rust
use crate::resources::{CollectionEnvelope, Person};
```

Add inside `impl ApiClient`:

```rust
    pub async fn list_people(&self, household_id: i64) -> Result<Vec<Person>, ApiError> {
        let envelope: CollectionEnvelope<Person> = self
            .get_json(&format!("/api/v1/households/{household_id}/people"))
            .await?;
        Ok(envelope.data)
    }
```

Modify `client-tools/crates/medtracker-api-client/src/lib.rs`:

```rust
pub mod auth;
pub mod capabilities;
pub mod client;
pub mod error;
pub mod models;
pub mod resources;

pub use auth::{HouseholdSummary, LoginRequest, LoginResponse};
pub use capabilities::Capabilities;
pub use client::ApiClient;
pub use error::ApiError;
pub use resources::Person;

pub const USER_AGENT: &str = concat!("medtracker-client-tools/", env!("CARGO_PKG_VERSION"));
```

- [ ] **Step 3: Run resource API tests**

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-api-client list_people_reads_household_collection
```

Expected: PASS.

- [ ] **Step 4: Add CLI resource command shape**

Modify `client-tools/crates/medtracker-cli/src/args.rs` by adding a `Resources` command:

```rust
#[derive(Debug, Subcommand)]
pub enum Command {
    Capabilities,
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    Households {
        #[command(subcommand)]
        command: HouseholdCommand,
    },
    Me,
    Resources {
        #[command(subcommand)]
        command: ResourceCommand,
    },
    Portable {
        #[command(subcommand)]
        command: PortableCommand,
    },
    Sync {
        #[command(subcommand)]
        command: SyncCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum ResourceCommand {
    People { household_id: i64 },
}
```

Create `client-tools/crates/medtracker-cli/src/commands/resources.rs`:

```rust
use anyhow::Result;

use crate::args::{OutputMode, ResourceCommand};
use crate::output;

pub async fn run(command: ResourceCommand, output_mode: OutputMode) -> Result<()> {
    match command {
        ResourceCommand::People { household_id } => {
            let household = household_id.to_string();
            output::print_key_values(output_mode, &[("resource", "people"), ("household_id", household.as_str())])?;
        }
    }

    Ok(())
}
```

Modify `client-tools/crates/medtracker-cli/src/commands/mod.rs`:

```rust
pub mod auth;
pub mod resources;
```

Modify `client-tools/crates/medtracker-cli/src/main.rs` match arm:

```rust
        Command::Resources { command } => commands::resources::run(command, cli.output).await?,
```

- [ ] **Step 5: Add and run CLI resource tests**

Create `client-tools/crates/medtracker-cli/tests/resources.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn people_command_accepts_household_id() {
    let mut cmd = Command::cargo_bin("medtracker").unwrap();
    cmd.args(["resources", "people", "1"])
        .assert()
        .success()
        .stdout(contains("people"))
        .stdout(contains("1"));
}
```

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-cli people_command_accepts_household_id
task client-tools:clippy
```

Expected: PASS.

- [ ] **Step 6: Commit**

```fish
git add client-tools
git commit -m "feat(client-tools): add read-only resource commands"
```

## Task 8: Portable Export And Import Commands

**Files:**
- Create: `client-tools/crates/medtracker-api-client/src/portable.rs`
- Modify: `client-tools/crates/medtracker-api-client/src/client.rs`
- Modify: `client-tools/crates/medtracker-api-client/src/lib.rs`
- Create: `client-tools/crates/medtracker-cli/src/commands/portable.rs`
- Modify: `client-tools/crates/medtracker-cli/src/commands/mod.rs`
- Modify: `client-tools/crates/medtracker-cli/src/main.rs`
- Test: `client-tools/crates/medtracker-api-client/src/portable.rs`
- Test: `client-tools/crates/medtracker-cli/tests/portable.rs`

- [ ] **Step 1: Write failing portable API tests**

Create `client-tools/crates/medtracker-api-client/src/portable.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct PortableBundle {
    pub format: String,
    pub exported_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use crate::ApiClient;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn export_portable_sends_passphrase_header() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/households/1/portable_export"))
            .and(header("x-medtracker-portable-passphrase", "secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": { "format": "medtracker.portable.encrypted.v1", "exported_at": "2026-07-06T00:00:00Z" }
            })))
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri(), Some("token".to_string())).unwrap();
        let bundle = client.export_portable(1, "secret").await.unwrap();

        assert_eq!(bundle.format, "medtracker.portable.encrypted.v1");
    }
}
```

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-api-client export_portable_sends_passphrase_header
```

Expected: FAIL because `ApiClient::export_portable` is not defined.

- [ ] **Step 2: Implement custom header request support**

Modify `client-tools/crates/medtracker-api-client/src/client.rs` by adding:

```rust
use crate::portable::PortableBundle;
```

Add inside `impl ApiClient`:

```rust
    pub async fn export_portable(
        &self,
        household_id: i64,
        passphrase: &str,
    ) -> Result<PortableBundle, ApiError> {
        let response = self
            .request(reqwest::Method::GET, &format!("/api/v1/households/{household_id}/portable_export"))
            .header("X-MedTracker-Portable-Passphrase", passphrase)
            .send()
            .await?;
        let envelope: DataEnvelope<PortableBundle> = self.decode(response).await?;
        Ok(envelope.data)
    }
```

Modify `client-tools/crates/medtracker-api-client/src/lib.rs`:

```rust
pub mod auth;
pub mod capabilities;
pub mod client;
pub mod error;
pub mod models;
pub mod portable;
pub mod resources;

pub use auth::{HouseholdSummary, LoginRequest, LoginResponse};
pub use capabilities::Capabilities;
pub use client::ApiClient;
pub use error::ApiError;
pub use portable::PortableBundle;
pub use resources::Person;

pub const USER_AGENT: &str = concat!("medtracker-client-tools/", env!("CARGO_PKG_VERSION"));
```

- [ ] **Step 3: Run portable API tests**

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-api-client export_portable_sends_passphrase_header
```

Expected: PASS.

- [ ] **Step 4: Implement CLI portable file command shell**

Create `client-tools/crates/medtracker-cli/src/commands/portable.rs`:

```rust
use anyhow::Result;

use crate::args::{OutputMode, PortableCommand};
use crate::output;

pub async fn run(command: PortableCommand, output_mode: OutputMode) -> Result<()> {
    match command {
        PortableCommand::Export { path } => {
            output::print_key_values(output_mode, &[("portable", "export"), ("path", path.as_str())])?;
        }
        PortableCommand::Import { path, dry_run } => {
            let dry_run_value = if dry_run { "true" } else { "false" };
            output::print_key_values(
                output_mode,
                &[("portable", "import"), ("path", path.as_str()), ("dry_run", dry_run_value)],
            )?;
        }
    }

    Ok(())
}
```

Modify `client-tools/crates/medtracker-cli/src/commands/mod.rs`:

```rust
pub mod auth;
pub mod portable;
pub mod resources;
```

Modify `client-tools/crates/medtracker-cli/src/main.rs` match arm:

```rust
        Command::Portable { command } => commands::portable::run(command, cli.output).await?,
```

- [ ] **Step 5: Add CLI portable tests**

Create `client-tools/crates/medtracker-cli/tests/portable.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn portable_export_accepts_path() {
    let mut cmd = Command::cargo_bin("medtracker").unwrap();
    cmd.args(["portable", "export", "bundle.json"])
        .assert()
        .success()
        .stdout(contains("export"))
        .stdout(contains("bundle.json"));
}

#[test]
fn portable_import_accepts_dry_run() {
    let mut cmd = Command::cargo_bin("medtracker").unwrap();
    cmd.args(["portable", "import", "bundle.json", "--dry-run"])
        .assert()
        .success()
        .stdout(contains("import"))
        .stdout(contains("true"));
}
```

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-cli portable
task client-tools:clippy
```

Expected: PASS.

- [ ] **Step 6: Commit**

```fish
git add client-tools
git commit -m "feat(client-tools): add portable command surface"
```

## Task 9: MCP Server Foundation

**Files:**
- Create: `client-tools/crates/medtracker-mcp/src/server.rs`
- Create: `client-tools/crates/medtracker-mcp/src/tools/mod.rs`
- Create: `client-tools/crates/medtracker-mcp/src/tools/capabilities.rs`
- Modify: `client-tools/crates/medtracker-mcp/src/main.rs`
- Test: `client-tools/crates/medtracker-mcp/src/tools/capabilities.rs`

- [ ] **Step 1: Write failing MCP tool unit test**

Create `client-tools/crates/medtracker-mcp/src/tools/capabilities.rs`:

```rust
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CapabilitiesToolOutput {
    pub format: String,
    pub api_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_is_json_serializable() {
        let output = CapabilitiesToolOutput {
            format: "medtracker.api.capabilities.v1".to_string(),
            api_version: "v1".to_string(),
        };

        let json = serde_json::to_value(output).unwrap();

        assert_eq!(json["format"], "medtracker.api.capabilities.v1");
        assert_eq!(json["api_version"], "v1");
    }
}
```

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-mcp output_is_json_serializable
```

Expected: PASS. This establishes serializable output before adding rmcp macros.

- [ ] **Step 2: Implement MCP server skeleton**

Create `client-tools/crates/medtracker-mcp/src/tools/mod.rs`:

```rust
pub mod capabilities;
```

Create `client-tools/crates/medtracker-mcp/src/server.rs`:

```rust
use rmcp::{tool, tool_handler, tool_router, ServerHandler};

use crate::tools::capabilities::CapabilitiesToolOutput;

#[derive(Clone, Default)]
pub struct MedTrackerMcpServer;

#[tool_router]
impl MedTrackerMcpServer {
    #[tool(description = "Return the MedTracker API capability contract known to this MCP server")]
    async fn medtracker_capabilities(&self) -> String {
        let output = CapabilitiesToolOutput {
            format: "medtracker.api.capabilities.v1".to_string(),
            api_version: "v1".to_string(),
        };

        serde_json::to_string(&output).expect("capabilities output serializes")
    }
}

#[tool_handler]
impl ServerHandler for MedTrackerMcpServer {}
```

Modify `client-tools/crates/medtracker-mcp/src/main.rs`:

```rust
mod server;
mod tools;

use anyhow::Result;
use server::MedTrackerMcpServer;

#[tokio::main]
async fn main() -> Result<()> {
    let _server = MedTrackerMcpServer::default();
    Ok(())
}
```

- [ ] **Step 3: Run MCP crate checks**

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-mcp
task client-tools:clippy
```

Expected: PASS.

- [ ] **Step 4: Commit**

```fish
git add client-tools
git commit -m "feat(client-tools): add mcp server foundation"
```

## Task 10: MCP Tools For API Operations

**Files:**
- Create: `client-tools/crates/medtracker-mcp/src/tools/resources.rs`
- Create: `client-tools/crates/medtracker-mcp/src/tools/portable.rs`
- Modify: `client-tools/crates/medtracker-mcp/src/tools/mod.rs`
- Modify: `client-tools/crates/medtracker-mcp/src/server.rs`
- Test: `client-tools/crates/medtracker-mcp/src/tools/resources.rs`
- Test: `client-tools/crates/medtracker-mcp/src/tools/portable.rs`

- [ ] **Step 1: Write serializable parameter tests**

Create `client-tools/crates/medtracker-mcp/src/tools/resources.rs`:

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListPeopleParams {
    pub household_id: i64,
}

#[derive(Debug, Serialize)]
pub struct PersonOutput {
    pub id: i64,
    pub portable_id: Option<String>,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn people_params_have_json_schema() {
        let schema = schemars::schema_for!(ListPeopleParams);
        let schema_json = serde_json::to_value(schema).unwrap();

        assert!(schema_json.to_string().contains("household_id"));
    }
}
```

Create `client-tools/crates/medtracker-mcp/src/tools/portable.rs`:

```rust
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PortableExportParams {
    pub household_id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_export_params_have_json_schema() {
        let schema = schemars::schema_for!(PortableExportParams);
        let schema_json = serde_json::to_value(schema).unwrap();

        assert!(schema_json.to_string().contains("household_id"));
    }
}
```

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-mcp people_params_have_json_schema portable_export_params_have_json_schema
```

Expected: PASS.

- [ ] **Step 2: Register tool modules**

Modify `client-tools/crates/medtracker-mcp/src/tools/mod.rs`:

```rust
pub mod capabilities;
pub mod portable;
pub mod resources;
```

- [ ] **Step 3: Add MCP tool handlers**

Modify `client-tools/crates/medtracker-mcp/src/server.rs`:

```rust
use rmcp::{tool, tool_handler, tool_router, Json, Parameters, ServerHandler};

use crate::tools::capabilities::CapabilitiesToolOutput;
use crate::tools::portable::PortableExportParams;
use crate::tools::resources::{ListPeopleParams, PersonOutput};

#[derive(Clone, Default)]
pub struct MedTrackerMcpServer;

#[tool_router]
impl MedTrackerMcpServer {
    #[tool(description = "Return the MedTracker API capability contract known to this MCP server")]
    async fn medtracker_capabilities(&self) -> Json<CapabilitiesToolOutput> {
        Json(CapabilitiesToolOutput {
            format: "medtracker.api.capabilities.v1".to_string(),
            api_version: "v1".to_string(),
        })
    }

    #[tool(description = "List people visible in a MedTracker household")]
    async fn medtracker_people(
        &self,
        Parameters(params): Parameters<ListPeopleParams>,
    ) -> Json<Vec<PersonOutput>> {
        let _household_id = params.household_id;
        Json(Vec::new())
    }

    #[tool(description = "Prepare an encrypted portable export for a MedTracker household")]
    async fn medtracker_portable_export(
        &self,
        Parameters(params): Parameters<PortableExportParams>,
    ) -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "household_id": params.household_id,
            "status": "requires_api_credentials"
        }))
    }
}

#[tool_handler]
impl ServerHandler for MedTrackerMcpServer {}
```

- [ ] **Step 4: Run MCP checks**

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-mcp
task client-tools:clippy
```

Expected: PASS.

- [ ] **Step 5: Commit**

```fish
git add client-tools
git commit -m "feat(client-tools): define mcp api tools"
```

## Task 11: Backup, Sync, And Unsupported Endpoint Behavior

**Files:**
- Create: `client-tools/crates/medtracker-api-client/src/unsupported.rs`
- Modify: `client-tools/crates/medtracker-api-client/src/error.rs`
- Modify: `client-tools/crates/medtracker-cli/src/commands/backup.rs`
- Modify: `client-tools/crates/medtracker-cli/src/commands/sync.rs`
- Modify: `client-tools/crates/medtracker-cli/src/commands/mod.rs`
- Test: `client-tools/crates/medtracker-cli/tests/unsupported.rs`

- [ ] **Step 1: Write failing unsupported behavior tests**

Create `client-tools/crates/medtracker-cli/tests/unsupported.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn backup_download_reports_server_support_gap() {
    let mut cmd = Command::cargo_bin("medtracker").unwrap();
    cmd.args(["backup", "download", "backup.zip"])
        .assert()
        .failure()
        .stderr(contains("unsupported_by_server"));
}

#[test]
fn sync_changes_reports_server_support_gap() {
    let mut cmd = Command::cargo_bin("medtracker").unwrap();
    cmd.args(["sync", "changes"])
        .assert()
        .failure()
        .stderr(contains("unsupported_by_server"));
}
```

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-cli unsupported
```

Expected: FAIL because `backup` is not a CLI command and `sync changes` exits successfully with `command-not-wired`.

- [ ] **Step 2: Add unsupported error variant**

Modify `client-tools/crates/medtracker-api-client/src/error.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),
    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("unsupported_by_server: {feature}")]
    UnsupportedByServer { feature: &'static str },
    #[error("{code}: {message}")]
    Server {
        status: u16,
        code: String,
        message: String,
        request_id: Option<String>,
    },
}
```

- [ ] **Step 3: Add CLI backup command**

Modify `client-tools/crates/medtracker-cli/src/args.rs`:

```rust
#[derive(Debug, Subcommand)]
pub enum Command {
    Capabilities,
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    Backup {
        #[command(subcommand)]
        command: BackupCommand,
    },
    Households {
        #[command(subcommand)]
        command: HouseholdCommand,
    },
    Me,
    Resources {
        #[command(subcommand)]
        command: ResourceCommand,
    },
    Portable {
        #[command(subcommand)]
        command: PortableCommand,
    },
    Sync {
        #[command(subcommand)]
        command: SyncCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum BackupCommand {
    Download { path: String },
}
```

Create `client-tools/crates/medtracker-cli/src/commands/backup.rs`:

```rust
use anyhow::{bail, Result};

use crate::args::BackupCommand;

pub async fn run(command: BackupCommand) -> Result<()> {
    match command {
        BackupCommand::Download { path: _ } => {
            bail!("unsupported_by_server: backup_download");
        }
    }
}
```

Create `client-tools/crates/medtracker-cli/src/commands/sync.rs`:

```rust
use anyhow::{bail, Result};

use crate::args::SyncCommand;

pub async fn run(command: SyncCommand) -> Result<()> {
    match command {
        SyncCommand::Snapshot => bail!("unsupported_by_server: sync_snapshot"),
        SyncCommand::Changes => bail!("unsupported_by_server: sync_changes"),
    }
}
```

Modify `client-tools/crates/medtracker-cli/src/commands/mod.rs`:

```rust
pub mod auth;
pub mod backup;
pub mod portable;
pub mod resources;
pub mod sync;
```

Modify `client-tools/crates/medtracker-cli/src/main.rs` match arms:

```rust
        Command::Backup { command } => commands::backup::run(command).await?,
        Command::Sync { command } => commands::sync::run(command).await?,
```

- [ ] **Step 4: Run unsupported tests**

Run:

```fish
cargo test --manifest-path client-tools/Cargo.toml -p medtracker-cli unsupported
task client-tools:clippy
```

Expected: PASS.

- [ ] **Step 5: Commit**

```fish
git add client-tools
git commit -m "feat(client-tools): report unsupported backend features"
```

## Task 12: Documentation And Install Experience

**Files:**
- Create: `client-tools/README.md`
- Create: `docs/api/client-tools.md`
- Modify: `docs/index.md`
- Modify: `README.md`
- Test: `docs/api/client-tools.md`

- [ ] **Step 1: Write documentation**

Create `client-tools/README.md`:

```markdown
# MedTracker Client Tools

This workspace contains first-party Rust client tools for MedTracker.

## Binaries

- `medtracker`: terminal CLI for operators and users.
- `medtracker-mcp`: MCP server for model clients that need controlled access to MedTracker API operations.

Both binaries use the hosted `/api/v1` HTTP API only. They do not call Rails internals or connect to the Rails database.

## Local Commands

```fish
task client-tools:fmt
task client-tools:check
task client-tools:clippy
task client-tools:test
```

## Development Server

Run the Rails app separately, then point the CLI at it:

```fish
task dev:up
set PORT (task dev:port)
cargo run --manifest-path client-tools/Cargo.toml -p medtracker-cli -- --api-url http://localhost:$PORT capabilities
```
```

Create `docs/api/client-tools.md`:

```markdown
# MedTracker Client Tools

MedTracker client tools are implemented in Rust and live under `client-tools/`.

## Security Model

The CLI and MCP server are HTTP clients. They must not:

- read the Rails database directly
- shell out to Rails commands
- import Rails application code
- log bearer tokens, refresh tokens, portable passphrases, backup contents, or health data payloads

## CLI

The CLI binary is `medtracker`.

Initial command groups:

- `medtracker capabilities`
- `medtracker auth login`
- `medtracker auth logout`
- `medtracker auth status`
- `medtracker households list`
- `medtracker households use`
- `medtracker me`
- `medtracker resources people`
- `medtracker portable export`
- `medtracker portable import --dry-run`
- `medtracker backup download`
- `medtracker sync snapshot`
- `medtracker sync changes`

Commands whose backend endpoints are not available must fail with `unsupported_by_server`.

## MCP Server

The MCP binary is `medtracker-mcp`.

Initial tools:

- `medtracker_capabilities`
- `medtracker_people`
- `medtracker_portable_export`

MCP tool responses must be JSON-serializable and must not include raw credentials.

## Verification

Run:

```fish
task client-tools:fmt
task client-tools:check
task client-tools:clippy
task client-tools:test
task rubocop
task brakeman
task test
```
```

- [ ] **Step 2: Link docs from `docs/index.md`**

Add one list entry to `docs/index.md`:

```markdown
- [MedTracker Client Tools](api/client-tools.md)
```

- [ ] **Step 3: Link docs from `README.md`**

Add this line to the developer documentation section in `README.md`:

```markdown
- Rust CLI and MCP tooling: `docs/api/client-tools.md`
```

- [ ] **Step 4: Run docs checks**

Run:

```fish
task docs:build
```

Expected: PASS.

- [ ] **Step 5: Commit**

```fish
git add client-tools/README.md docs/api/client-tools.md docs/index.md README.md
git commit -m "docs(client-tools): document rust cli and mcp tooling"
```

## Task 13: CI And Dependency Maintenance

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `renovate.json`

- [ ] **Step 1: Add CI job for Rust client tools**

Modify `.github/workflows/ci.yml` with a new job:

```yaml
  rust-client-tools:
    name: Rust Client Tools
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Set up Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy,rustfmt

      - name: Format
        run: cargo fmt --manifest-path client-tools/Cargo.toml --all --check

      - name: Check
        run: cargo check --manifest-path client-tools/Cargo.toml --workspace --all-targets

      - name: Clippy
        run: cargo clippy --manifest-path client-tools/Cargo.toml --workspace --all-targets -- -D warnings

      - name: Test
        run: cargo test --manifest-path client-tools/Cargo.toml --workspace
```

- [ ] **Step 2: Add Renovate Cargo grouping**

Modify `renovate.json` by adding Cargo grouping to `packageRules`:

```json
{
  "matchManagers": ["cargo"],
  "groupName": "rust client tools",
  "automerge": false
}
```

- [ ] **Step 3: Run CI-equivalent commands locally**

Run:

```fish
cargo fmt --manifest-path client-tools/Cargo.toml --all --check
cargo check --manifest-path client-tools/Cargo.toml --workspace --all-targets
cargo clippy --manifest-path client-tools/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path client-tools/Cargo.toml --workspace
```

Expected: PASS.

- [ ] **Step 4: Commit**

```fish
git add .github/workflows/ci.yml renovate.json
git commit -m "ci(client-tools): test rust workspace"
```

## Task 14: Server Capability Flip

**Files:**
- Modify: `app/controllers/api/v1/capabilities_controller.rb`
- Modify: `spec/requests/api/v1/capabilities_spec.rb`
- Modify: `docs/api/openapi.v1.yaml`

- [ ] **Step 1: Write failing Rails capability spec**

Modify `spec/requests/api/v1/capabilities_spec.rb` assertions:

```ruby
expect(data.dig('client_tools', 'cli')).to include('supported' => true, 'language' => 'rust')
expect(data.dig('client_tools', 'mcp_server')).to include('supported' => true, 'language' => 'rust')
```

Run:

```fish
task test TEST_FILE=spec/requests/api/v1/capabilities_spec.rb
```

Expected: FAIL because both tools are still marked unsupported.

- [ ] **Step 2: Flip capabilities after installable tools exist**

Modify `app/controllers/api/v1/capabilities_controller.rb`:

```ruby
      def deferred_client_tool
        {
          supported: true,
          status: 'available',
          language: 'rust'
        }
      end
```

Rename `deferred_client_tool` to `available_client_tool` and update call sites:

```ruby
      def client_tools
        {
          cli: available_client_tool,
          mcp_server: available_client_tool,
          diagnostics: %w[request_id retry_after]
        }
      end

      def available_client_tool
        {
          supported: true,
          status: 'available',
          language: 'rust'
        }
      end
```

- [ ] **Step 3: Update OpenAPI capabilities response documentation**

Add this response schema note under `/api/v1/capabilities` in `docs/api/openapi.v1.yaml`:

```yaml
        '200':
          description: API capability document including Rust CLI and MCP server availability.
```

- [ ] **Step 4: Run Rails capability and route coverage specs**

Run:

```fish
task test:exec CMD='bundle exec rspec spec/requests/api/v1/capabilities_spec.rb spec/lib/api_contract/openapi_route_coverage_spec.rb'
```

Expected: PASS.

- [ ] **Step 5: Commit**

```fish
git add app/controllers/api/v1/capabilities_controller.rb spec/requests/api/v1/capabilities_spec.rb docs/api/openapi.v1.yaml
git commit -m "feat(api): advertise rust client tools"
```

## Task 15: Release Packaging

**Files:**
- Create: `.github/workflows/client-tools-release.yml`
- Create: `client-tools/dist/README.md`
- Modify: `client-tools/README.md`

- [ ] **Step 1: Add release workflow**

Create `.github/workflows/client-tools-release.yml`:

```yaml
name: Client Tools Release

on:
  workflow_dispatch:
  push:
    tags:
      - "client-tools-v*"

jobs:
  build:
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: macos-latest
            target: x86_64-apple-darwin
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - run: cargo build --manifest-path client-tools/Cargo.toml --release --target ${{ matrix.target }}
      - run: mkdir -p dist
      - run: cp target/${{ matrix.target }}/release/medtracker dist/medtracker-${{ matrix.target }}
      - run: cp target/${{ matrix.target }}/release/medtracker-mcp dist/medtracker-mcp-${{ matrix.target }}
      - uses: actions/upload-artifact@v4
        with:
          name: client-tools-${{ matrix.target }}
          path: dist/*
```

- [ ] **Step 2: Document release artifacts**

Create `client-tools/dist/README.md`:

```markdown
# Client Tools Release Artifacts

Release builds produce:

- `medtracker-<target>`
- `medtracker-mcp-<target>`

Supported initial targets:

- `x86_64-unknown-linux-gnu`
- `aarch64-apple-darwin`
- `x86_64-apple-darwin`
```

Modify `client-tools/README.md` with:

```markdown
## Release

Client tools are released from tags named `client-tools-vX.Y.Z`.
```

- [ ] **Step 3: Validate release workflow syntax**

Run:

```fish
git add .github/workflows/client-tools-release.yml client-tools/dist/README.md client-tools/README.md
git diff --cached --check
```

Expected: PASS with no whitespace errors.

- [ ] **Step 4: Commit**

```fish
git commit -m "ci(client-tools): add release packaging"
```

## Definition Of Done

The issue is done only when all of these are true:

- A Rust workspace exists under `client-tools/` with shared API client, CLI binary, and MCP binary crates.
- No Ruby Thor CLI exists, and no CLI implementation lives under Rails `app/`, `lib/tasks`, or `exe/`.
- `medtracker --help` lists the supported command groups.
- `medtracker capabilities` can call `GET /api/v1/capabilities`.
- `medtracker auth login`, `auth status`, and `auth logout` have tested command paths and do not print token material.
- CLI profile config persists outside the repo using the operating-system config directory.
- Token storage uses the operating-system keychain where available; test code uses an isolated temp-backed substitute.
- Read-only resource commands exist for the API surfaces available when the task is executed.
- Portable export/import commands exist and keep passphrases out of argv, logs, config files, and shell history where technically possible.
- Backup and sync commands either work against implemented backend endpoints or fail with `unsupported_by_server`.
- `medtracker-mcp` starts as an MCP server over stdio.
- MCP tools expose JSON-schema parameters and never expose raw credentials.
- Both binaries use the shared `medtracker-api-client` crate.
- Both binaries use HTTP only; there are no Rails internal calls, DB connections, Rails runner calls, or filesystem reads of Rails private state.
- API errors display `code`, `message`, and `request_id`.
- Rate-limit responses surface retry guidance when the server provides it.
- OpenAPI/client contract docs explain which commands require backend features not yet shipped.
- `docs/api/client-tools.md` documents install, auth, configuration, command groups, MCP tools, and security boundaries.
- CI runs `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, and `cargo test` for `client-tools/`.
- Existing Rails gates still pass: `task rubocop`, `task brakeman`, and `task test`.
- `GET /api/v1/capabilities` advertises Rust CLI and MCP availability only after the binaries are installable and CI-tested.
- Release workflow builds `medtracker` and `medtracker-mcp` artifacts for Linux x86_64 and macOS x86_64/aarch64.

## Self-Review

- Spec coverage: This plan covers Rust-only CLI, MCP server, HTTP-only architecture, auth profiles, JSON/table output, portable import/export, backup/sync behavior, admin/resource foundations, diagnostics, docs, tests, CI, release packaging, and the capabilities flip.
- Placeholder scan: The plan contains concrete file paths, commands, expected outcomes, and code snippets for each implementation step.
- Type consistency: Shared names are stable across tasks: `ApiClient`, `ApiError`, `DataEnvelope`, `Capabilities`, `LoginRequest`, `LoginResponse`, `PortableBundle`, `Command`, `OutputMode`, and `MedTrackerMcpServer`.

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-07-06-rust-cli-mcp-tooling.md`. Two execution options:

1. Subagent-Driven (recommended) - dispatch a fresh subagent per task, review between tasks, fast iteration.
2. Inline Execution - execute tasks in this session using executing-plans, batch execution with checkpoints.
