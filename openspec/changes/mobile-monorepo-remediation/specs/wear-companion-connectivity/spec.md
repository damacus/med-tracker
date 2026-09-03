## Purpose

Provides a privacy-preserving Wear OS connectivity foundation in which the paired Android phone remains the only owner of Rails networking and authenticated credentials.

## ADDED Requirements

### Requirement: Wear discovers a compatible phone companion
The phone SHALL advertise capability `medtracker_phone_companion_v1`, and the Wear application SHALL distinguish a missing application, disconnected device, incompatible protocol, signed-out session, and ready companion.

#### Scenario: Compatible signed-in phone
- **WHEN** the watch discovers a connected phone advertising the supported capability and receives a compatible signed-in status
- **THEN** the watch displays the ready state

#### Scenario: Phone disconnects
- **WHEN** a previously discovered phone becomes unreachable
- **THEN** the watch displays the disconnected state without attempting direct Rails access

### Requirement: Companion status persists across disconnection
The phone SHALL publish `/medtracker/companion/status` through the Wearable Data Layer with protocol version, phone application version, session state, and publication time.

#### Scenario: Status published while disconnected
- **WHEN** the phone publishes a status while the devices cannot communicate
- **THEN** the status is synchronised after reconnection and the watch converges to the new state

### Requirement: Wear owns no Rails credentials or transport
The Wear application SHALL contain no Rails base URL, access token, refresh token, password flow, or generated Rails API client.

#### Scenario: Wear application inspection
- **WHEN** the Wear application dependencies, manifest, resources, and persisted data are inspected
- **THEN** no direct Rails transport or credential storage capability is present
