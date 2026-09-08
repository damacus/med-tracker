## Why

[Issue #1982](https://github.com/damacus/med-tracker/issues/1982) needs a way to explain stock lost without administration. An absolute inventory correction makes users calculate the remaining total and does not identify the quantity removed as a distinct event.

## What Changes

- Add a web stock-removal form for authorised inventory managers, with quantity, stock source, reason and optional note.
- Subtract from the selected tracked stock atomically, rejecting invalid quantities and insufficient stock.
- Preserve a distinct, attributable stock-removal audit event and make duplicate submissions safe.
- Retain existing inventory corrections, refills and dose recording.

## Capabilities

### New Capabilities

- `stock-removals`: Remove tracked stock without creating administration history.

### Modified Capabilities

None.

## Impact

Rails inventory services, web routes, Phlex forms, audit evidence and translated UI text. Reuse existing stock records and audit storage; no migration or new dependency is required.

## Non-goals

Automatic stock tracking (#1979), scheduled-dose outcomes (#1980), native write APIs, offline removal queues, household transfers and changes to inventory permissions are excluded.
