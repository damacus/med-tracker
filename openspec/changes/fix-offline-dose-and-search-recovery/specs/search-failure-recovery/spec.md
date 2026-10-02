## Purpose

Distinguish unavailable search from valid empty results and let users recover without losing their query.

## ADDED Requirements

### Requirement: Search failures are recoverable and distinct
Search SHALL show an accessible, translated failure state for transport, HTTP and malformed responses, retain the query, and offer retry. Aborted searches SHALL NOT show a failure.

#### Scenario: Failed request
- **GIVEN** a non-empty search query
- **WHEN** the request fails or returns invalid data
- **THEN** an error and retry action appear instead of a no-results message

#### Scenario: Retry succeeds
- **GIVEN** a failed search
- **WHEN** the user retries and the request succeeds
- **THEN** results for the retained query replace the error

#### Scenario: Empty successful result
- **GIVEN** a non-empty query
- **WHEN** a successful response contains an empty results list
- **THEN** the normal no-results state appears
