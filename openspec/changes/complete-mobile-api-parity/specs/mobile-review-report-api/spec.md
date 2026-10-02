## Purpose

Let native clients manage medicine reviews and retrieve protected clinical reports with the web's access and date rules.

## ADDED Requirements

### Requirement: Manage the review queue
The API SHALL provide bounded review listings with existing status/priority/hidden filters and authorized ETag-protected practitioner review updates, preserving evidence snapshots.

#### Scenario: Review a prompt
- **GIVEN** an authorized reviewer and current prompt
- **WHEN** review status and practitioner context are submitted
- **THEN** review attribution is saved and immutable evidence is unchanged

#### Scenario: Reject stale or inaccessible updates
- **GIVEN** a stale ETag or inaccessible prompt
- **WHEN** a review update is submitted
- **THEN** no review change commits and no hidden context is disclosed

### Requirement: Retrieve protected reports
The API SHALL return typed JSON and downloadable PDF health-history and medicine-review reports using existing person permissions and date limits, with no-store responses.

#### Scenario: Export an authorized report
- **GIVEN** a permitted selected person and valid date range
- **WHEN** the client requests JSON or PDF
- **THEN** the report contains only permitted records and records the applicable download audit

#### Scenario: Reject excessive or invalid ranges
- **GIVEN** an invalid date or range beyond the existing report maximum
- **WHEN** a report is requested
- **THEN** a stable validation error is returned

#### Scenario: Handle rendering failure
- **GIVEN** a valid authorized PDF request whose renderer fails
- **WHEN** the API handles the failure
- **THEN** it returns a stable error without internal exception text or clinical data
