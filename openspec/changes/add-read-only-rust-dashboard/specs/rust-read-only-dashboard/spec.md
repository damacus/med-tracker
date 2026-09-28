## Purpose

Provide a usable, visually faithful dashboard for authorised users to inspect medication information without performing clinical writes.

## ADDED Requirements

### Requirement: Existing authentication protections

The application SHALL provide email/password sign-in, validation errors, session expiry handling and sign-out while preserving existing MFA restrictions and keeping credentials out of browser storage.

#### Scenario: Successful sign-in and sign-out
- **GIVEN** an eligible account with valid credentials
- **WHEN** the user signs in and subsequently signs out
- **THEN** the dashboard is available only during the authenticated session.

#### Scenario: Invalid credentials, restricted account or expired session
- **GIVEN** invalid credentials, an account requiring unsupported MFA completion, or an expired session
- **WHEN** dashboard access is attempted
- **THEN** access is denied or redirected to sign-in with a safe explanation and no medical data.

### Requirement: Faithful responsive dashboard

The dashboard SHALL match the supplied light Material You reference, including branding, sidebar, search affordance, greeting/date, identity, selector, three summary metrics, schedule, stock inventory, Smart Insights surface and application version.

#### Scenario: Desktop and mobile presentation
- **GIVEN** deterministic authorised fixture data
- **WHEN** the dashboard is viewed at desktop and mobile widths
- **THEN** all required surfaces are present, readable and keyboard accessible without horizontal overflow.

### Requirement: Accurate authorised medication information

The dashboard SHALL derive people, next due, due now, tasks left, routine and as-needed medication, stock levels and today's recorded doses from real authorised data using existing medication and display-timezone rules. It SHALL display populated, empty, completed, paused and not-taken states where the Rails reference does.

#### Scenario: Medication states and day boundaries
- **GIVEN** authorised records spanning routine/as-needed, completed, paused and not-taken states and a fixed clock near a day boundary
- **WHEN** the dashboard is rendered
- **THEN** metrics, task states and conditional recorded-dose history match the intended Rails rules for the displayed day.

#### Scenario: Empty or failed reads
- **GIVEN** no applicable records or a failed required read
- **WHEN** the dashboard loads
- **THEN** it shows an honest empty or error state without presenting fabricated values as successful data.

#### Scenario: Restricted visibility
- **GIVEN** records belonging to another household or an inaccessible person
- **WHEN** a user views or attempts to select those records
- **THEN** neither records nor derived counts are disclosed.

### Requirement: Persistent read-only selection

The dashboard SHALL support authorised person selection, All Family, keyboard interaction and disclosures. Selection SHALL be represented in the URL and survive refresh and browser history navigation.

#### Scenario: Selection and navigation
- **GIVEN** multiple visible people
- **WHEN** a user selects a person or All Family using the keyboard and navigates back, forward or refreshes
- **THEN** the URL and displayed authorised data remain consistent with the selected history entry.

#### Scenario: Invalid selection
- **GIVEN** a malformed or inaccessible person selection in the URL
- **WHEN** the page is requested
- **THEN** a safe error is shown without exposing records or counts for the invalid selection.

### Requirement: Clearly unavailable actions

Add Person, Add Medication, dose actions, refill ordering, report links, search and non-dashboard navigation SHALL remain visible but unavailable with accessible disabled semantics and a concise explanation. Smart Insights SHALL say “Insights coming soon”.

#### Scenario: Attempted unavailable action
- **GIVEN** the read-only dashboard
- **WHEN** a user focuses or attempts to activate an unavailable control
- **THEN** its unavailable status is understandable and no clinical mutation, dead-link navigation or simulated success occurs.

#### Scenario: Insights placeholder
- **GIVEN** any authorised dashboard state
- **WHEN** the Smart Insights card is displayed
- **THEN** it shows “Insights coming soon” and does not claim learning or analysis has occurred.
