## Purpose

Make the read-only dashboard installable while ensuring offline behaviour never stores or reveals private medication information.

## ADDED Requirements

### Requirement: Installable application shell

The application SHALL provide a valid installable PWA shell with branding, icons and a supported start URL.

#### Scenario: Installation and launch
- **GIVEN** a supported browser and secure origin
- **WHEN** the user installs and launches the application
- **THEN** the shell opens the application and applies normal authentication requirements.

### Requirement: Public assets only in caches

Only explicitly allowed public static assets SHALL be cached. Authenticated HTML, API responses and medical data SHALL remain network-only and private, including after sign-out or session expiry.

#### Scenario: Authenticated browsing and sign-out
- **GIVEN** a user who has viewed medication records
- **WHEN** browser caches are inspected after browsing and sign-out
- **THEN** no authenticated pages, API responses or medical data have been stored for offline reuse.

### Requirement: Safe offline recovery

Offline navigation SHALL display a patient-free reconnect screen instead of a stale medical dashboard. Reconnection SHALL restore access only through the normal authentication boundary.

#### Scenario: Connection loss and recovery
- **GIVEN** an application previously used by an authenticated user
- **WHEN** connectivity is lost and a navigation is attempted, then restored
- **THEN** the offline screen contains no patient information and renewed access checks session validity before displaying data.
