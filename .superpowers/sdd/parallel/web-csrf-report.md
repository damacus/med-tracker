# Platform and web profile CSRF contract report

Base: `ebef1e39` (contract stack HEAD when this isolated worktree was created).
Branch: `codex/axum-api-web-csrf`.

## Change

- The Rails contract runner enables its existing test-only CSRF overlay for focused and full `platform` and `web_profile` targets. The combined profile/web profile run recreates the web container when entering the overlay.
- Existing unauthenticated and unauthorized probes provide valid CSRF tokens, so their expected results still test authentication and authorization.
- A platform settings mutation without a token and app token creation with missing or wrong tokens must redirect to login. Independent public read-backs show that the attempted settings and token writes did not happen.
- The runner isolation mock expects container recreation at CSRF overlay boundaries.

No Rails application code changed.

## Verification

- Red before the runner change: focused Rails platform test returned `302` rather than expected CSRF `303` for the missing-token mutation. Early standalone probes also hit the shared login throttle; these were folded into existing signed-in tests.
- Green with the overlay: `task contract:platform-rails` 4/4 and `task contract:web-profile-rails` 7/7; both isolated Docker projects cleaned.
- `task contract:platform-rust` 0/4 and `task contract:web-profile-rust` 0/7 against the absent Rust target at `127.0.0.1:39999`. Both failed on connection as intended, before assertions; both isolated projects cleaned.
- `task contract:fmt`, `task contract:clippy`, Fish syntax, runner isolation mock, and `git diff --check` passed.
- Broad `task contract:isolation` reached a real Docker phase under the sandbox and failed on Docker socket permission. The focused runner isolation mock passed; this broad command is not counted as a green gate.

The web JSON actions CSRF follow-up is intentionally outside this commit; it will use the integrated target on the later base.
