## 1. Inventory removal

- [x] 1.1 Red: add service specs for quantity validation, source selection, audit attribution, rollback, stale stock and replay; observe failure with task test TEST_FILE=spec/services/remove_medication_stock_service_spec.rb.
- [x] 1.2 Green and refactor: implement atomic removal using existing inventory and audit boundaries; pass the service specs.

## 2. Web workflow

- [x] 2.1 Red: add request and browser coverage for authorised success, forbidden and cross-household access, source tampering, errors and history; observe failures with the focused task test commands.
- [x] 2.2 Green and refactor: add the form, routes, history and translations; pass focused request and browser checks, including desktop/mobile screenshots.

## 3. Delivery

- [x] 3.1 Verify integration using task rubocop, task test, task docs:build, strict OpenSpec validation and git diff --check; record any environmental limits.
- [x] 3.2 Publish the verified change in a pull request linked to #1982; verify remote branch and CI status.
