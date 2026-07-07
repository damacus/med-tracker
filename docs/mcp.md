# MedTracker MCP

MedTracker exposes an authenticated Model Context Protocol (MCP) endpoint at
`/mcp`. It uses Streamable HTTP and the same bearer credentials as the mobile
API: API sessions and API app tokens.

Use the MCP server when an MCP-capable assistant needs read-only medication
context for a household. The endpoint is not a replacement for the REST API, and
it does not support writes.

## Prerequisites

- A running MedTracker deployment reachable over HTTPS. Local development can
  use `http://localhost:<port>`.
- A MedTracker account with active access to the household you want the client
  to read.
- Access to complete multi-factor authentication when your account policy
  requires it.
- `curl` and `jq` for the verification commands below.
- An MCP client that supports hosted HTTP MCP servers, such as Codex, Claude
  Code, or VS Code.

## Create an API app token

API app tokens are the recommended credential for MCP clients because they are
revocable and scoped to one household membership.

1. Sign in to MedTracker with the account that should own the MCP credential.
2. From the household dashboard, open your profile. The direct URL is
   `/households/<household_slug>/profile`.
3. In the **API Tokens** card, enter a clear **Token name**, such as
   `Laptop Codex MCP`.
4. Select **Create token**.
5. If MedTracker asks for two-factor authentication, complete it, return to the
   profile page, and select **Create token** again.
6. Copy the token immediately from the success message. MedTracker only shows
   the raw token once.

Set the deployment URL and token in your shell:

```bash
export MEDTRACKER_URL="https://medtracker.example.com"
export MEDTRACKER_MCP_TOKEN="paste-token-here"
```

For local development, use the dev server URL instead:

```bash
export MEDTRACKER_URL="http://localhost:$(task dev:port)"
export MEDTRACKER_MCP_TOKEN="paste-token-here"
```

Do not put bearer tokens in URLs, screenshots, checked-in config, or shell
history shared with other people.

## Verify the endpoint

First confirm the deployment advertises the MCP server:

```bash
curl -sS "$MEDTRACKER_URL/api/v1/capabilities" | jq '.data.client_tools.mcp_server'
```

Expected fields include:

```json
{
  "supported": true,
  "transport": "streamable_http",
  "endpoint": "/mcp"
}
```

Then list the MCP tools directly. Streamable HTTP requests use JSON-RPC over
`POST /mcp`; the bearer token must be sent in the `Authorization` header on
every request.

```bash
curl -sS "$MEDTRACKER_URL/mcp" \
  -H "Authorization: Bearer $MEDTRACKER_MCP_TOKEN" \
  -H "Accept: application/json, text/event-stream" \
  -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | jq
```

The response should include the read-only MedTracker tools listed below.

## Configure clients

### Codex

Codex can read the bearer token from an environment variable:

```bash
codex mcp add medtracker --url "$MEDTRACKER_URL/mcp" --bearer-token-env-var MEDTRACKER_MCP_TOKEN
codex mcp get medtracker
```

Export `MEDTRACKER_MCP_TOKEN` before starting Codex or restart any already
running Codex session after exporting it. A running process cannot read
environment variables that were created after it started.

### Claude Code

Claude Code can connect to hosted HTTP MCP servers and pass a static bearer
header:

```bash
claude mcp add --transport http medtracker "$MEDTRACKER_URL/mcp" \
  --header "Authorization: Bearer $MEDTRACKER_MCP_TOKEN"
claude mcp get medtracker
```

Use `/mcp` inside Claude Code to inspect the connection and available tools.
The command above stores the expanded header in Claude Code's local MCP
configuration, so use a private user/local scope and do not commit that config
to a repository.

### VS Code

Open the MCP user or workspace configuration and add a hosted HTTP server. Use
an input variable so the API token is not stored in the JSON file.

```json
{
  "inputs": [
    {
      "type": "promptString",
      "id": "medtracker-token",
      "description": "MedTracker API app token",
      "password": true
    }
  ],
  "servers": {
    "medtracker": {
      "type": "http",
      "url": "https://medtracker.example.com/mcp",
      "headers": {
        "Authorization": "Bearer ${input:medtracker-token}"
      }
    }
  }
}
```

After saving the file, start or restart the server from VS Code's MCP controls
and approve the server when VS Code asks whether you trust it.

### Claude Desktop

Do not assume the Claude Code command above applies to Claude Desktop. Claude
Desktop MCP configuration support varies by release and is often documented
around local `stdio` servers. If your Claude Desktop build or organization
connector settings support hosted HTTP MCP servers, use the same endpoint and
`Authorization: Bearer <token>` header values shown above. Otherwise, use Codex,
Claude Code, VS Code, or a local bridge that forwards to the hosted MedTracker
endpoint.

## Use the MCP server

The server currently exposes these read-only tools:

| Tool | Purpose |
| --- | --- |
| `medtracker_current_user` | Returns the authenticated API user profile. |
| `medtracker_household_snapshot` | Returns a policy-scoped portable household snapshot. |
| `medtracker_today_schedule` | Returns visible schedules and medications taken today. |
| `medtracker_inventory_risks` | Returns visible medication inventory risks. |
| `medtracker_health_history_summary` | Returns a bounded recent health-history summary. |

The health-history tool defaults to the last 30 days and rejects ranges over
180 days.

Call a tool directly:

```bash
curl -sS "$MEDTRACKER_URL/mcp" \
  -H "Authorization: Bearer $MEDTRACKER_MCP_TOKEN" \
  -H "Accept: application/json, text/event-stream" \
  -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"medtracker_today_schedule","arguments":{}}}' | jq
```

Call the bounded health-history summary:

```bash
curl -sS "$MEDTRACKER_URL/mcp" \
  -H "Authorization: Bearer $MEDTRACKER_MCP_TOKEN" \
  -H "Accept: application/json, text/event-stream" \
  -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"medtracker_health_history_summary","arguments":{"start_date":"2026-07-01","end_date":"2026-07-07"}}}' | jq
```

The MCP resource `medtracker://household/snapshot` returns the same structured
payload as `medtracker_household_snapshot`:

```bash
curl -sS "$MEDTRACKER_URL/mcp" \
  -H "Authorization: Bearer $MEDTRACKER_MCP_TOKEN" \
  -H "Accept: application/json, text/event-stream" \
  -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","id":4,"method":"resources/read","params":{"uri":"medtracker://household/snapshot"}}' | jq
```

The prompt `medtracker_household_review` gives an agent a safe review brief for
visible schedules, inventory risk, and recent health history:

```bash
curl -sS "$MEDTRACKER_URL/mcp" \
  -H "Authorization: Bearer $MEDTRACKER_MCP_TOKEN" \
  -H "Accept: application/json, text/event-stream" \
  -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","id":5,"method":"prompts/list","params":{}}' | jq
```

In an MCP client, ask for bounded, review-oriented work, for example:

- "Use MedTracker to summarize today's medication schedule."
- "Use MedTracker to list low-stock or out-of-stock medicines."
- "Use the `medtracker_household_review` prompt to prepare a caregiver review."

## Troubleshooting

| Symptom | What to check |
| --- | --- |
| `401 Unauthorized` | Confirm the `Authorization: Bearer ...` header is present, the token was copied correctly, the token has not been revoked, and the account, user, and household membership are active. |
| `429 Too Many Requests` | The MCP endpoint is rate limited. Wait for the number of seconds in the `Retry-After` response header before retrying. |
| `JSON-RPC body must be a single request object` | Send one JSON-RPC request object per HTTP request. Batch arrays are rejected. |
| The client cannot see tools | Check `/api/v1/capabilities`, then list tools with the direct `curl` command to separate client configuration problems from server problems. |
| The client still sends an old or empty token | Restart the client after changing environment variables. For VS Code, update the stored input value or reset the MCP server. |
| A token worked before but now fails | The token may have been revoked, the account may be locked, or the household membership may no longer be active. Create a new API app token from the profile page after resolving the account state. |

## Security boundary

MCP access is tenant-scoped by the bearer credential's household membership.
The endpoint rejects revoked credentials, expired API sessions, locked-out
accounts, inactive users, inactive memberships, and stale API credentials.

Tools use the same Pundit policy scopes as the API and never receive the raw
bearer token. Every authenticated MCP request writes a `SecurityAuditEvent` with
`event_type: mcp.request`, request ID, IP address, household, actor account,
actor membership, JSON-RPC method, outcome, and HTTP status.

Revoke unused MCP tokens from the profile page. Treat MCP clients as trusted
integrations because they can read the same household medication context that
the token holder can access.

## When not to use MCP

Do not use the MCP for writes, medication administration, clinical source of
truth exports, or workflows that require formal API versioning. Use the REST API
for mobile sync and any state change. Use the encrypted portable export/import
flow for backup, migration, and restore workflows.

## Protocol and client references

- [Model Context Protocol Streamable HTTP transport](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)
- [Model Context Protocol authorization](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization)
- [Claude Code MCP documentation](https://code.claude.com/docs/en/mcp)
- [VS Code MCP configuration reference](https://code.visualstudio.com/docs/agents/reference/mcp-configuration)
