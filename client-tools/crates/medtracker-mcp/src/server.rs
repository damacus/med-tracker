use std::io::{self, BufRead, Write};

use anyhow::{Context, Result, bail};
use medtracker_api_client::ApiClient;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::tools::{HouseholdParams, ResourceListParams, SyncChangesParams, tool_list};

#[derive(Debug, Deserialize)]
struct Request {
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

pub async fn run_stdio() -> Result<()> {
    let base_url = std::env::var("MEDTRACKER_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    let token = std::env::var("MEDTRACKER_TOKEN").ok();
    let client = ApiClient::new(base_url, token)?;
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line.context("failed to read stdio request")?;
        if line.trim().is_empty() {
            continue;
        }

        let response = match handle_line(&client, &line).await {
            Ok(response) => response,
            Err(error) => error_response(None, error.to_string()),
        };
        writeln!(stdout, "{response}")?;
        stdout.flush()?;
    }

    Ok(())
}

async fn handle_line(client: &ApiClient, line: &str) -> Result<Value> {
    let request: Request = serde_json::from_str(line).context("invalid JSON-RPC request")?;
    let id = request.id.clone();
    let result = match request.method.as_str() {
        "initialize" => json!({
            "protocolVersion": "2025-06-18",
            "serverInfo": {
                "name": "medtracker-mcp",
                "version": env!("CARGO_PKG_VERSION")
            },
            "capabilities": {
                "tools": {}
            }
        }),
        "tools/list" => json!({ "tools": tool_list() }),
        "tools/call" => call_tool(client, request.params).await?,
        method => bail!("unsupported MCP method: {method}"),
    };

    Ok(json!({
        "jsonrpc": request.jsonrpc.unwrap_or_else(|| "2.0".to_string()),
        "id": id,
        "result": result
    }))
}

async fn call_tool(client: &ApiClient, params: Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .context("tools/call requires name")?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let value = match name {
        "medtracker_capabilities" => json_value(client.capabilities().await?),
        "medtracker_households" => client.households().await?,
        "medtracker_me" => {
            let args: HouseholdParams = serde_json::from_value(arguments)?;
            client.me(&args.household_id).await?
        }
        "medtracker_resource_list" => {
            let args: ResourceListParams = serde_json::from_value(arguments)?;
            client
                .list_resource(&args.household_id, args.kind.into())
                .await?
        }
        "medtracker_portable_export" => {
            let args: HouseholdParams = serde_json::from_value(arguments)?;
            client.portable_export(&args.household_id).await?
        }
        "medtracker_sync_snapshot" => {
            let args: HouseholdParams = serde_json::from_value(arguments)?;
            client.sync_snapshot(&args.household_id).await?
        }
        "medtracker_sync_changes" => {
            let args: SyncChangesParams = serde_json::from_value(arguments)?;
            client
                .sync_changes(&args.household_id, args.since.as_deref())
                .await?
        }
        other => return Err(anyhow::anyhow!("unsupported MCP tool: {other}")),
    };

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&value)?
            }
        ]
    }))
}

fn json_value<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serializable API model")
}

fn error_response(id: Option<Value>, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": -32603,
            "message": message
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tools_list_returns_schema_definitions() {
        let client = ApiClient::new("http://localhost:3000", None).unwrap();
        let response = handle_line(&client, r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#)
            .await
            .unwrap();

        assert_eq!(
            response["result"]["tools"][0]["name"],
            "medtracker_capabilities"
        );
        assert!(
            response["result"]["tools"][3]["inputSchema"]["properties"]
                .get("household_id")
                .is_some()
        );
    }
}
