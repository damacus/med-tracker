## Purpose

Let native inventory managers record attributable stock loss and retrieve its history without logging a dose.

## ADDED Requirements

### Requirement: Record stock removed
The API SHALL accept positive decimal-string quantity, selected stock source, supported reason, optional note and stable submission identity, with the same stock rules as the web.

#### Scenario: Remove tracked stock
- **GIVEN** an authorized inventory manager and sufficient tracked stock
- **WHEN** the client records a removal
- **THEN** quantity decreases once and attributable history is created without a take

#### Scenario: Replay without double removal
- **GIVEN** a committed removal
- **WHEN** the same request is retried
- **THEN** the original result is returned without another decrement

#### Scenario: Reject invalid source or amount
- **GIVEN** an inaccessible source, insufficient stock or invalid quantity
- **WHEN** the client records removal
- **THEN** no stock, audit or sync change commits and the error reveals no hidden data

### Requirement: Read bounded stock loss history
The API SHALL expose paginated authorized removal history with quantity, reason, optional note, time and permitted actor attribution.

#### Scenario: List removal events
- **GIVEN** accessible medication with more events than one page
- **WHEN** the client reads history
- **THEN** bounded results and continuation metadata preserve event order
