# Web JSON actions CSRF contract report

Base: `c83b93d9` (integrated web JSON actions contract commit).
Branch: `codex/axum-api-web-json-csrf`.

## Change

- The focused, feature-disabled, and full Rails contract runners load the existing test-only CSRF overlay alongside the AI suggestion adapter for web JSON actions.
- Web JSON contract helpers send the session CSRF token in the `X-CSRF-Token` header. Existing unauthenticated, unauthorized, success, validation, and throttle probes supply a valid token so each continues to test its intended boundary.
- New probes require JSON `401` for missing and wrong tokens on paid AI suggestions and person deletion. An independent authorized session reads the household snapshot after each rejected deletion and verifies the person list is unchanged.

No Rails application code changed.

## Verification

- Red before enabling the overlay: missing token returned `200` for paid AI suggestions and `422` for person deletion, rather than `401`. A first pass also found that restricted fixture members cannot view the profile page; token retrieval now uses their authorized dashboard. Extra fixture owner sessions initially hit the login throttle; independent read-back uses other fixture memberships.
- Green with the overlay: `task contract:web-json-actions-rails` 11/11 (one intentionally ignored feature-disabled test); `task contract:web-json-actions-disabled-rails` 1/1.
- `task contract:web-json-actions-rust` 0/11 against the absent target at `127.0.0.1:39999`, all failing on connection before assertions. All isolated Docker projects cleaned.
- `task contract:fmt`, `task contract:clippy`, Fish syntax, the runner isolation mock, and `git diff --check` passed.
