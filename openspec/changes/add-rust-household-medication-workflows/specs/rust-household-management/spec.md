# Rust Household Management

## Purpose

Let authorised household members manage people, locations, medication inventory and treatment plans through complete Rust browser journeys while retaining existing medication safety rules.

## ADDED Requirements

### Requirement: People management
The application SHALL provide authorised people list, detail, creation and editing with existing person-type and capacity rules.

#### Scenario: Add and edit a person
- **GIVEN** a member permitted to manage people
- **WHEN** they create a person, edit their details and reload
- **THEN** persisted details and available assignment choices reflect the change.

#### Scenario: Invalid or inaccessible person
- **GIVEN** an invalid draft or a person outside the member's access
- **WHEN** a browser form is submitted
- **THEN** validation preserves entered values without writing, and inaccessible records are denied without exposing their details.

### Requirement: Location management
The application SHALL provide authorised location list, detail, creation and editing, including permitted stock contents.

#### Scenario: Immediately usable location
- **GIVEN** a member permitted to create locations
- **WHEN** they create a location and open medication creation
- **THEN** the new location is selectable and saved medication appears in its authorised contents.

### Requirement: Medication management
The application SHALL support manual medication creation and editing using existing validation, dosage options and units.

#### Scenario: Save and return to inventory
- **GIVEN** valid medication details and a permitted location
- **WHEN** the member saves and reloads inventory
- **THEN** the persisted medication appears and is available for assignment and stock actions.

#### Scenario: Rejected medication
- **GIVEN** an invalid medication draft
- **WHEN** the member submits it
- **THEN** errors are associated with fields, submitted values remain available and no success notice is shown.

### Requirement: Stock actions
The application SHALL expose authorised inventory adjustment, ordering and receipt actions using the shared API's arithmetic, transaction and replay rules.

#### Scenario: Adjust and receive stock
- **GIVEN** permitted medication and valid quantities
- **WHEN** adjustment, ordering and receipt are completed
- **THEN** reloaded balance and status match committed writes and required audit/sync evidence exists.

#### Scenario: Conflict or repeated submission
- **GIVEN** a stale or repeated mutation protected by the API
- **WHEN** the action is submitted
- **THEN** the browser preserves the API conflict/replay outcome without duplicating stock or silently overwriting it.

### Requirement: Treatment management
The application SHALL support permitted direct assignments and all existing scheduled treatment types, including daily, multiple daily, weekly, specific dates, PRN, tapering and every-other-day, with edit and pause/resume history.

#### Scenario: Create and use treatment
- **GIVEN** an authorised person, medication and supported dose
- **WHEN** a treatment is reviewed and saved
- **THEN** its type, dose and dates survive reload and the dashboard permits only eligible administration.

#### Scenario: Taper boundary and pause
- **GIVEN** a multi-step taper and manage permission
- **WHEN** a step boundary is crossed or treatment is paused and resumed
- **THEN** administration uses the effective step and paused intervals/history remain truthful.

### Requirement: Authorised and accessible rendering
The application SHALL keep tenant/person authorisation authoritative, provide keyboard-accessible desktop/mobile forms and render supported translations consistently across SSR and hydration.

#### Scenario: Forged request
- **GIVEN** an expired session, invalid CSRF or foreign record identifier
- **WHEN** a read or mutation is attempted
- **THEN** access is denied without a clinical write or private-data disclosure.

#### Scenario: Locale and validation
- **GIVEN** any supported locale and a rejected form
- **WHEN** the member navigates by keyboard at desktop or mobile width
- **THEN** labels, errors and notices are localised, associated with controls, and initial/edit values remain intact.

#### Scenario: Offline privacy
- **GIVEN** previous authenticated medication browsing
- **WHEN** connectivity is lost or the member signs out
- **THEN** caches contain no authenticated HTML, API records or medical drafts.

#### Scenario: Unknown validation text
- **GIVEN** a rejected form with an unmapped API validation message
- **WHEN** it is rendered in any supported locale
- **THEN** a useful translated generic error is associated with the affected field or summary, submitted values remain intact and raw unknown API text is not exposed.

#### Scenario: Concurrent first dosage option
- **GIVEN** a scalar medication edit and first dosage-option insertion before or between its record/options reads
- **WHEN** the stale form is submitted
- **THEN** 409 preserves every submitted scalar draft value without writing, and the final API mutation retains the original submitted If-Match rather than the newly observed version.

#### Scenario: Limited-member dashboard without personal profile access
- **GIVEN** an active authenticated member with a valid grant to another person and no own-person profile grant
- **WHEN** the dashboard receives 403/404 for optional personal preferences
- **THEN** the dashboard uses safe defaults and only authorised clinical data, without granting profile access or exposing mutation controls to a view-only member.
