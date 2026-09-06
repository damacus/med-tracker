# Native iOS notification delivery

The household native-device registration endpoint accepts an optional
`apns_environment` of `sandbox` or `production`. The signed iOS app must send
the environment that matches its APNs entitlement. Registration retries update
the same account-owned token; they cannot take ownership of another account's
token. Revocation remains idempotent and uses the token string as the resource
identifier.

Existing registrations without an environment keep the configured server APNs
host. Explicit environments select Apple's sandbox or production host, rather
than the legacy host override. The configured APNs bundle identifier must still
match the app that issued the token. No credentials belong in client requests.

APNs alerts use a generic title and body, regardless of the detailed message
used for other channels. The custom `path` and `kind` fields carry navigation:

| Kind | Native destination |
| --- | --- |
| `dose_due`, `missed_dose` | Today in the addressed household |
| `low_stock` | Medicines in the addressed household |
| `test`, `unknown` | Today, if the path names an accessible household |

The supported path is `/households/<slug>/dashboard`. Clients must authenticate
and match the selected household before navigating. Payloads are not authority
to switch households, reveal a person, or record a dose.

Low-stock delivery accepts either a native device token or a web-push
subscription. Existing notification preference, eligibility and duplicate
suppression rules remain in force.

Deploy this server change before enabling iOS native notifications. Schema,
server deployment, APNs configuration and physical-device validation require
their normal separate approvals; this change does not perform them.
