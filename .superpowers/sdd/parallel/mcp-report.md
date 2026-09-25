# MCP HTTP contract report

## Scope and status

This branch adds target-independent black-box contracts for the retained `/mcp` mount. It changes Rust contract tests, their Task runner, and the parity matrix. It does not change Rails product code, schema, native clients, or OpenAPI. The seven focused cases run against disposable Rails/PostgreSQL 18 households; the absent Rust target remains expected red. The integration owner will run the combined corpus and resolve shared-file overlaps.

## Rails source behavior inventoried

- `/mcp` mounts `MedTrackerMcp::RackApp` with the MCP Ruby gem's stateless Streamable HTTP transport and JSON responses. Authenticated JSON-RPC POST requests support `initialize`, `tools/list`, `tools/call`, `resources/list`, `resources/read`, `prompts/list`, and `prompts/get`. `notifications/initialized` returns HTTP 202 without a body. Authenticated GET returns 405; authenticated DELETE returns 200. JSON-RPC batch arrays return HTTP 400.
- The server advertises exactly five read-only tools: current user, household snapshot, today's schedule, inventory risks, and bounded health-history summary. It also advertises one household snapshot resource and one household review prompt. The tests call representative tools and both additional surfaces without making clinical writes.
- `/mcp` accepts valid API session and API app-token bearer credentials bound to an active household membership. It rejects missing, revoked, locked, and expired credentials. Rails exposes no separate `/mcp` HTTP consent operation: the holder authorizes MCP access by issuing and presenting a household-scoped bearer. Client-side approval is outside the server HTTP contract.
- The household snapshot tool uses the mobile snapshot's **manage** person scope. A member with only a view grant cannot see the managed patient in that snapshot; the owner with a manage grant can. The today-schedule and health-history tools use view policy scope, so the view-granted member can see the managed patient's records. Hidden and foreign fixture identities remain excluded. A foreign bearer reads its own household, not the primary fixture's household.
- Protocol failures are structured: unknown method `-32601`, missing tool name and unknown resource `-32602`, bounded history error as a tool result with `isError: true`, missing/revoked/locked/expired bearer as HTTP 401 JSON, and cross-origin request as HTTP 403 JSON. Successful authenticated requests create request-correlated `mcp.request` audit events with actor membership, method, outcome, and status; the public audit event excludes the raw app token.

## Red, green, and verification

- Initial focused contract passed the protocol inventory test. The expanded suite was red because it assumed a view grant appeared in the mobile snapshot; the Rails behavior is manage-scoped. After correcting that assertion, six cases passed.
- A transport probe was red on a guessed DELETE 405; Rails returned 200. The updated seven-case suite passed. A later exact error-code probe was red on a guessed unknown-resource `-32002`; Rails returned `-32602`. The final contract records Rails' output.
- `rtk task contract:mcp-rails`: final run 7 passed, 0 failed. It provisions disposable PostgreSQL 18 households and removes its Docker project after the run.
- `rtk task contract:mcp-rust`: expected red, 0 passed and 7 failed to connect to absent `127.0.0.1:39999/mcp`. This is not a Rails failure or an implemented Rust parity result.
- `rtk task contract:fmt`, `rtk task contract:clippy`, `rtk proxy fish -n rust/contract-tests/run.fish`, and `rtk git diff --check`: passed.
- `rtk task docs:build` could not download locked `zensical==0.0.63` because package-host DNS was unavailable in this isolated checkout; the other cached UV environment also lacked its wheel. Running the same installed Zensical build executable from the read-only active checkout, with this branch as working directory, returned `No issues found`. That fallback checked this branch's Markdown without changing the active checkout, but it does not count as a passing Task wrapper invocation.

## Limits

The contract checks HTTP-visible scope and audit data. It does not prove internal gem implementation, physical database rollback, or interactive consent UX. The app-token creation UX is outside this MCP HTTP slice. The full combined suite and Rust parity remain integration-owner work.
