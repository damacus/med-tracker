# Android build and release configuration

The Android phone application is an independent Gradle project. Run its checks from the repository root with `task android:ci`.

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

The release client intentionally retains the `password_login` capability metadata field and its enum because they describe server capability data. That inert metadata is not a password authentication interface. Release compilation excludes the generated `AuthLoginRequest` model and `createLoginSession` operation; the full generated password transport exists only in the shared non-release source set used by explicitly labelled debug and staging variants.

The live canary is also staging-only. It runs only through `task android:integration:canary`, requires its three `MEDTRACKER_CANARY_*` environment values, and supplies the explicit Gradle opt-in property. Ordinary `./gradlew test` and pull-request CI skip it even if canary environment values happen to exist.
