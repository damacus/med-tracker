# Android build and release configuration

The Android phone application is an independent Gradle project. Run its checks from the repository root with `task android:ci`.

## Wear companion

Open `mobile/android` in Android Studio to discover `:phone`, `:wear`, and the pure Kotlin `:wear-protocol` module. `task android:projects` also lists these modules. Wear displays connectivity only; it has no medication UI, Rails client, credential storage or direct network transport.

The phone advertises `medtracker_phone_companion_v1` and persistently publishes `/medtracker/companion/status`. Version 1 contains only `protocolVersion`, `phoneAppVersion`, `sessionState` (`signed_in` or `signed_out`) and `publishedAt` (Unix milliseconds). Encoding is deterministic UTF-8 JSON. Missing or malformed data cannot make Wear ready. An unsupported version displays an incompatible-app state.

Phone publication observes the same application-owned session as the phone UI. Wear queries both known and reachable capability nodes, reads persistent data when opened, and refreshes on Data Layer events. While visible, it retries discovery every five seconds. Status never overrides missing or unreachable capability discovery. Publication failures retry against the current session, so a later sign-out does not leave an earlier failed sign-in queued indefinitely.

Phone and watch share application ID `io.damacus.medtracker`, with matching `.debug` and `.staging` suffixes and the same local debug signing key. Both release APKs remain unsigned; release signing assets are not supplied here. Google Play services requires matching package names and signing identities for Data Layer communication.

`task android:ci` includes protocol/phone/Wear tests, phone/Wear lint and assemblies, release security inspection, Wear source/dependency/APK boundary checks, and pinned API regeneration checks. It does not contact a live Rails environment or start emulators.

To run the device smoke, first start an existing phone/Wear emulator pair with Google Play services and pair it in Android Studio. Use a clean phone emulator without the debug application installed. No emulator is created or downloaded by the task.

```fish
task android:wear:smoke PHONE_SERIAL=emulator-5554 WATCH_SERIAL=emulator-5556
```

The command checks the explicit device types, app IDs and signing certificates before installation. It refuses physical devices and existing phone sessions, installs matching debug applications, opens the phone while Wear is stopped, then checks the watch for signed-out status. It restarts the watch process and checks that persistent status loads again. The command leaves the test applications installed. Emulator serials above are examples, not device discovery defaults.

The paired-device smoke and screenshots remain unverified until a paired Wear emulator is available. Hermetic tests cover disconnection, malformed or unsupported status, reconnect, and later persistent-data convergence.

API references: [Data Layer security and availability](https://developer.android.com/training/wearables/data/overview), [persistent data items](https://developer.android.com/training/wearables/data/data-items), [capability discovery](https://developers.google.com/android/reference/com/google/android/gms/wearable/CapabilityClient), and [Wearable SDK 20.0.1 release notes](https://developers.google.com/android/guides/releases#april_28_2026).

Ordinary CI assembles release with deterministic invalid values so it can verify compilation and security boundaries without inventing live identity configuration. Those APKs are not deployable. A real package requires all approved values below:

| Gradle property | Packaging environment value | Purpose |
|---|---|---|
| `medtracker.release.serverUrl` | `MEDTRACKER_RELEASE_SERVER_URL` | Fixed Rails server URL |
| `medtracker.release.oidcAuthorizationEndpoint` | `MEDTRACKER_RELEASE_OIDC_AUTHORIZATION_ENDPOINT` | OIDC authorization endpoint |
| `medtracker.release.oidcTokenEndpoint` | `MEDTRACKER_RELEASE_OIDC_TOKEN_ENDPOINT` | OIDC token endpoint |
| `medtracker.release.oidcClientId` | `MEDTRACKER_RELEASE_OIDC_CLIENT_ID` | Public Android OIDC client identifier |
| `medtracker.release.oidcRedirectUri` | `MEDTRACKER_RELEASE_OIDC_REDIRECT_URI` | Registered Android redirect URI |
| `medtracker.release.oidcRedirectScheme` | `MEDTRACKER_RELEASE_OIDC_REDIRECT_SCHEME` | Manifest redirect scheme matching the URI |

Set the six packaging environment values, then run:

```fish
task android:package:release
```

`task android:release:validate` and the packaging command fail closed when a value is missing, empty, contains `invalid`, uses a non-HTTPS server or endpoint, or has a redirect scheme mismatch. Packaging also checks the produced dex files for the password-login request descriptor, generated operation, and `/auth/login` path.

All variants use the same editable instance URL and Rodauth browser sign-in.
The phone discovers endpoints and public client configuration from the selected
instance. The retained release packaging validation above still runs; its
provider endpoint/client settings are no longer used by the sign-in journey.
Callback registration must match the app's registered redirect URI.

The obsolete `password_login` capability, generated password transport,
ID-token exchange, selection grant and custom refresh operation are removed.
There is one generated API surface for all variants. Household switching uses
the account token and loads current authorised households.

The live canary check is staging-only. It runs through
`task android:integration:canary` with `MEDTRACKER_CANARY_BASE_URL` and
`MEDTRACKER_CANARY_TOKEN`, a current token obtained through browser login.
It verifies discovery and household access, not interactive SSO itself.
The explicit Gradle opt-in is required. Ordinary tests and pull-request CI skip
it even if those environment values exist.
