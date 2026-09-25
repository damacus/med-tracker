# Secure journey entry: runner report

## Runner and network prerequisites

The canonical command is `task api:acceptance`; it provisions PostgreSQL 18, the Rails fixture source, and the Rust API in one disposable Compose project. For the frozen combined run, set `CONTRACT_TEST_SUBNET` to a checked free IPv4 `/28` and `CONTRACT_BROWSER_TESTS=true` to include the Playwright sidecar. The HTTP and browser sidecars share the Rust API network namespace and call `127.0.0.1:39998`; the Compose runner publishes no API host port.

The previously used `192.168.240.0/28` range was occupied by an active Docker `/20`. Immediately before the final run, `task api:contract-subnet-check CONTRACT_TEST_SUBNET=192.168.64.0/28` passed host-route and Docker-overlap checks. The final project used `192.168.64.0/28` and was cleaned by the Task runner.

## Final combined acceptance

- Task: `CONTRACT_TEST_SUBNET=192.168.64.0/28 CONTRACT_BROWSER_TESTS=true task api:acceptance` (environment set in Fish).
- Project: `mtcontract-9bee0f80a25e4d2f`.
- HTTP results: forecast 3/3, existing mobile medication OAuth 7/7, medication read 9/9, secure OAuth journey 11/11; 30 passed, 0 failed.
- Browser results: 5 passed, 0 failed, 0 skipped. Public login passed at desktop and mobile sizes; wrong-password submission retained a visible error and keyboard focus; login and consent completed at both desktop and mobile sizes.
- The test authorized the registered public client with S256 PKCE, denied password-only access for the account enrolled in OTP, rejected CSRF-less login and consent submissions, exchanged and rotated/revoked tokens, and checked current household medication access and foreign-household denial.
- The fixture file was `tmp/contract-tests/run.hLt4gJ/fixture.json`, 47,133 bytes, SHA-256 `6f64b3a868da9f2237a2ef805bcac72342cf1e49bf62ccade880a38c45a3328a`. Only its path, size and digest were retained; fixture contents were removed by cleanup.
- Runtime input manifest: 1,499 paths across Rails application/configuration and fixture inputs, root container/task wiring, Rust API/web/contract sources, manifests and locks, runner overlays, browser image and test, and fixture provisioning. Pre- and post-run SHA-256 matched at `36e580245342cce33ec66813f652cb504536185fa48398399334bcce22d13b59`; HEAD remained `0b6fa2a86a4d95e5c8fc7f70b1701bd35251a347`. The manifest included untracked `rust/api/src/oauth.rs`, `rust/web/src/auth.css`, both browser Dockerfile files, and the subnet/runner overlays.
- Immutable image evidence from BuildKit: Rust runner manifest-list SHA-256 `3069f54daf2d98b93742be320724e561321806a1867eb3440dee13a3bfbc7616`, config SHA-256 `c570d5d0f6a169a8204f8690ed2cd4e070285b1cbd9bf249ef90f41fa7a89d36`; browser manifest-list SHA-256 `3b95e1c4b0a8b122c4bc4efafa4693744f80aaa415bb8bb5fb0c397f56217005`, config SHA-256 `1a43dee1b5b39dc798101591c9d9eaa255c23f679074d6569a46e392789f04b5`; Rails test image manifest-list SHA-256 `5f4e80b1633e41c57ff93c5caff2166c046b3ef2c29a3a8f6b5bf2323bf5048e`, config SHA-256 `6705fa8523fa2d5dbebc5a0983a1a6dc12995534c587fa5b5411581f4c17c785`. The runner and browser images were project-tagged and removed during cleanup.

## Browser inspection and retained evidence

I inspected the screenshots after the first browser run exposed that the login page rendered without styling and the consent screen showed raw OAuth scopes. The product owner added and linked `auth.css`, then replaced the raw scopes with Rails-aligned labels and descriptions; the final screenshots show the styled public entry, authorization login, login error and consent pages at desktop/mobile widths. Inputs and labels fit, the primary action is clear, the consent descriptions are readable, the error is visible, and keyboard focus is visible. The screenshots also show the stylesheet took effect; the browser smoke suite does not separately assert the stylesheet response status.

Screenshots are in `docs/screenshots/journey-auth/`: `login-public-desktop.png`, `login-public-mobile.png`, `login-authorization-desktop.png`, `login-authorization-mobile.png`, `login-error-desktop.png`, `consent-desktop.png`, and `consent-mobile.png`. The obsolete unstyled screenshots from the first failed browser attempt were removed after correction.

The initial stable HTTP run passed 30/30. Its first browser attempt passed the two public-login checks and exposed three test-only errors (`Locator.isFocused is not a function`). The test owner replaced those invalid calls with active-element checks. A later run passed 30/30 HTTP and 5/5 browser checks, then the final run above verified the Rails-aligned consent copy with 30/30 HTTP and 5/5 browser checks. An earlier pre-freeze launch stopped before contract tests because sandbox access denied host routes and the Docker socket; it was owner-cleaned and is not counted as an acceptance result.

## Evidence limits

This proves the secure entry and medication-read portion of the journey. It does not cover dose creation, stock mutation/history, every authentication credential flow, or a complete medication application journey. The browser intercepted the native callback and did not contact an external identity provider. Successful access to the API is established from the internal sidecars; absence of a host port is Compose configuration evidence.
