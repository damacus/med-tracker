## 1. Sync recovery

- [x] 1.1 Add browser regressions for transient/auth/malformed failures, atomic failure storage and tenant isolation; observe failures before implementation.
- [x] 1.2 Preserve retryable doses, atomically retain rejections and show recovery actions; pass focused browser specs.

## 2. Pending stock

- [ ] 2.1 Reproduce selection of locally exhausted stock and overlapping clicks in browser tests.
- [ ] 2.2 Select using pending consumption and guard submission; pass focused browser specs.

## 3. Cached eligibility

- [ ] 3.1 Add failing request and browser tests for restricted sources, view-only access, stale metadata and pending overlap.
- [ ] 3.2 Expose server eligibility and effective dose, consume it conservatively in the browser and pass focused checks.

## 4. Search recovery

- [ ] 4.1 Add failing browser tests for HTTP/network/malformed failures and retry.
- [ ] 4.2 Render translated accessible errors and retry the retained query; pass focused search specs.

## 5. Delivery

- [ ] 5.1 Run OpenSpec validation, applicable lint/full Rails tests, documentation build and diff checks; record outcomes.
- [ ] 5.2 Capture desktop/mobile UI evidence and verify four published stacked PRs against their intended parents.
