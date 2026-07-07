# MedTracker MCP

MedTracker exposes an authenticated Model Context Protocol endpoint at `/mcp`.
It uses the Ruby MCP SDK Streamable HTTP transport and the same bearer
credentials as the mobile API: API sessions and API app tokens.

## Why MCP exists

The MCP gives agents a small, explicit read-only surface for household medication
context. It is for assisted review workflows where an agent needs current
MedTracker context without scraping HTML pages or learning the full REST API.

Good MCP use cases:

- Summarize today's visible medication schedule for the authenticated household membership.
- Check low-stock and out-of-stock medication context before a caregiver review.
- Review recent health history across people the membership is allowed to see.
- Fetch a portable household snapshot for offline or agent-side analysis.

## Endpoint

Use JSON-RPC over Streamable HTTP:

```http
POST /mcp
Authorization: Bearer <api-session-or-api-app-token>
Accept: application/json
Content-Type: application/json
```

Capabilities are advertised from `GET /api/v1/capabilities` under
`data.client_tools.mcp_server`.

## Tools

The server currently exposes read-only tools:

| Tool | Purpose |
| --- | --- |
| `medtracker_current_user` | Returns the authenticated API user profile. |
| `medtracker_household_snapshot` | Returns a policy-scoped portable household snapshot. |
| `medtracker_today_schedule` | Returns visible schedules and medications taken today. |
| `medtracker_inventory_risks` | Returns visible medication inventory risks. |
| `medtracker_health_history_summary` | Returns a bounded recent health-history summary. |

The health-history tool defaults to the last 30 days and rejects ranges over
180 days.

## Resources and prompts

The MCP resource `medtracker://household/snapshot` returns the same structured
payload as `medtracker_household_snapshot`.

The prompt `medtracker_household_review` gives an agent a safe review brief for
visible schedules, inventory risk, and recent health history.

## Security boundary

MCP access is tenant-scoped by the bearer credential's household membership.
The endpoint rejects revoked credentials, expired API sessions, locked-out
accounts, inactive users, inactive memberships, and stale API credentials.

Tools use the same Pundit policy scopes as the API and never receive the raw
bearer token. Every authenticated MCP request writes a `SecurityAuditEvent` with
`event_type: mcp.request`, request ID, IP address, household, actor account,
actor membership, JSON-RPC method, outcome, and HTTP status.

## When not to use MCP

Do not use the MCP for writes, medication administration, clinical source of
truth exports, or workflows that require formal API versioning. Use the REST API
for mobile sync and any state change. Use the encrypted portable export/import
flow for backup, migration, and restore workflows.
