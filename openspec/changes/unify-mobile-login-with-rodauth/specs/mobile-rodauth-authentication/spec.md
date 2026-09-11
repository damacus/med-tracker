## Purpose

Allow first-party mobile applications to use the installation's existing sign-in methods through one standard authorization flow while preserving session security.

## ADDED Requirements

### Requirement: Installation-owned mobile sign-in
First-party mobile applications SHALL use the installation's authorization-code flow with S256 PKCE and a platform browser authentication session. Local and delegated sign-in SHALL retain existing configuration, optional MFA enrolment, available MFA methods and admin policy.

#### Scenario: Local installation
- **GIVEN** no external provider is configured
- **WHEN** an eligible user signs in from a phone
- **THEN** existing local authentication methods complete the mobile authorization flow without requiring an external provider or MFA enrolment beyond existing policy

#### Scenario: Mixed authentication installation
- **GIVEN** both local and delegated authentication are configured
- **WHEN** a user chooses either method from the phone's browser sign-in journey
- **THEN** that method remains available under existing policy and returns to the same mobile authorization flow

#### Scenario: Delegated SSO
- **GIVEN** delegated authentication is configured and the browser has a provider session that satisfies policy
- **WHEN** the user chooses delegated login
- **THEN** the flow can reuse that session without the phone collecting provider credentials

### Requirement: Verified code redemption
Authorization codes SHALL be short-lived, single-use, bound to the registered client, exact redirect URI and S256 challenge, and redeemed only with the matching verifier. Upstream identity validation SHALL retain issuer, audience, expiry, nonce and signature checks.

#### Scenario: Invalid redemption
- **GIVEN** an issued authorization code
- **WHEN** redemption uses a wrong or missing verifier, wrong client, wrong redirect URI, expired code or previously redeemed code
- **THEN** no new usable credential is issued and a privacy-safe error is returned

#### Scenario: Concurrent redemption
- **GIVEN** a valid code and matching verifier
- **WHEN** two requests attempt redemption concurrently
- **THEN** at most one creates a usable credential

#### Scenario: Persistence failure
- **GIVEN** valid redemption inputs
- **WHEN** credential persistence fails
- **THEN** no successful credential response or partially usable session is exposed and retry behaviour respects single-use code semantics

### Requirement: Device session protections
Mobile credentials SHALL retain refresh rotation and replay protection, device session visibility and individual revocation, account eligibility checks and lockout enforcement. Refresh or issuance SHALL NOT falsely mark earlier MFA as newly performed.

#### Scenario: Device revocation and refresh replay
- **GIVEN** two active devices for one account
- **WHEN** one device is revoked or a spent refresh token is replayed
- **THEN** the affected session is rejected according to refresh replay policy without revoking an unrelated device solely because it shares the client registration

#### Scenario: Authentication freshness
- **GIVEN** an earlier MFA-authenticated session
- **WHEN** SSO or refresh issues a new mobile credential
- **THEN** sensitive-action checks use the verified original authentication evidence and do not treat token issuance time as fresh MFA

### Requirement: Instance URL discovery
Mobile applications SHALL support a user-selected instance URL and labelled canary/demo presets. From that URL they SHALL discover the installation's authorization endpoints and public platform client configuration without requiring users to enter provider URLs or secrets.

#### Scenario: Discover a configured instance
- **GIVEN** the user enters a supported instance URL or selects a preset
- **WHEN** the app prepares sign-in
- **THEN** it loads that instance's authorization metadata and platform client configuration and starts installation-owned login

#### Scenario: Reject inconsistent discovery
- **GIVEN** an instance returns unsupported or inconsistent issuer/endpoints
- **WHEN** the app validates discovery
- **THEN** it displays a connection error without sending existing credentials to a different instance

#### Scenario: Change instance
- **GIVEN** the phone is signed into instance A
- **WHEN** the user selects instance B
- **THEN** authentication state is isolated and A's credentials are never reused for B

### Requirement: Direct replacement and private diagnostics
Capabilities and API documentation SHALL advertise only available flows. The obsolete first-party ID-token exchange and selection-grant flow SHALL be removed in this delivery without a client compatibility period, because no active mobile clients require migration. Existing restricted integrations SHALL remain supported. Codes, verifiers and tokens SHALL be excluded from logs, traces and audit data.

#### Scenario: Old mobile exchange removed
- **GIVEN** the new authentication flow is delivered
- **WHEN** a caller attempts the retired mobile ID-token exchange
- **THEN** it cannot issue a credential and discovery advertises only the replacement flow

#### Scenario: Failed authentication diagnostics
- **GIVEN** an authentication request contains secrets and an invalid provider response
- **WHEN** authentication fails
- **THEN** response and diagnostics expose neither credential material nor private provider payloads
