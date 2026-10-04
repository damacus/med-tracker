# Rust Profile Page

## ADDED Requirements

### Requirement: One canonical Profile page

The Rust browser SHALL expose the signed-in account at `/households/{slug}/profile` with Profile, Security, Notifications and Advanced sections, using the Rails page's navigation and responsive layout.

#### Scenario: Change the selected section

- **WHEN** the person chooses a section by pointer or tab keyboard keys
- **THEN** the shared Profile shell remains visible, the selected section becomes active, and keyboard focus follows the selected tab

### Requirement: Working Profile controls

The Profile section SHALL show current personal information and SHALL let an authorised person update time zone, photo, Gravatar opt-in and mobile shortcuts through the authoritative API. Appearance choices SHALL remain active across sign-out and sign-in pages.

#### Scenario: Save time zone

- **WHEN** a person with manage access chooses a valid zone in the dialog and saves
- **THEN** the API stores it, the page confirms success, and reload displays the saved zone

#### Scenario: Preserve a Rails time-zone preference

- **WHEN** an account has a Rails label such as `London` and saves the Profile form
- **THEN** the API accepts and returns that exact label, while calendar calculations use its corresponding IANA zone

#### Scenario: View-only profile

- **WHEN** a person has view access but no manage grant
- **THEN** the page displays current values without an enabled save action, and a forged write is denied

### Requirement: Working account sections

Security SHALL provide verified email, password, authenticator, recovery-code and passkey flows. Notifications SHALL provide preferences, browser push, managed people and reminder times. Advanced SHALL provide tokens, exports, experiments, system information and account closure. All writes SHALL require the signed-in account, trusted origin, CSRF and the matching backend authorisation.

#### Scenario: Read back an account change

- **WHEN** an authorised control saves successfully
- **THEN** reopening its section shows the persisted value and a failed write does not report success

#### Scenario: Consume a secret once

- **WHEN** a recovery code, email verification key or one-time token is used
- **THEN** the backend applies its intended operation once without showing a reusable secret on later page loads

### Requirement: Rails appearance and modal flow

The Profile page SHALL offer all ten Rails colour themes with their locally served fonts, Light, Dark and System appearance, and themed surfaces and controls across all four tabs and account subflows. Dialogs SHALL dim the background using the foreground colour at ten percent opacity and blur it by 1.5 pixels.

#### Scenario: Change appearance

- **WHEN** a person selects a colour theme and Light or Dark appearance
- **THEN** every Profile tab and its dialogs use that palette, readable action text and matching Rails font, and navigation preserves the preference

#### Scenario: Follow the system appearance

- **WHEN** System appearance is selected and the operating system changes its colour scheme
- **THEN** the page follows the new scheme without changing the saved theme

#### Scenario: Dismiss a modal

- **WHEN** a person opens a Profile dialog
- **THEN** background scrolling is locked, Escape and backdrop clicks dismiss the dialog, and dismissal releases scrolling and restores focus to the trigger
