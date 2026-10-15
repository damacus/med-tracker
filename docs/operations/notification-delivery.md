# Notification Delivery Configuration

MedTracker can send notifications through email or browser push. Native mobile
delivery uses Apple Push Notification service (APNs) or Firebase Cloud Messaging
(FCM). Each channel is optional, but every setting for an enabled channel must
be present.

Store provider secrets in the deployment platform's secret store or Rails
encrypted credentials. Do not write tokens, private keys, subscription
endpoints, or notification content to operator logs.

## Email

Email is used for invitations and account-security messages. Configure the
`SMTP_*` variables and sender address described in [Mail setup](../mail-setup.md).

## Browser push

Browser push uses a VAPID key pair.

| Variable | Required | Purpose |
| --- | --- | --- |
| `VAPID_PUBLIC_KEY` | yes | Public key sent to the browser when it creates a subscription. |
| `VAPID_PRIVATE_KEY` | yes | Private signing key used by the notification sender. |
| `VAPID_SUBJECT` | no | Contact email without the `mailto:` prefix. Defaults to `notifications@example.com`. |

The public and private keys must belong to the same VAPID key pair. The private
key is a secret. A missing public key prevents the browser from creating a
working subscription.

### Loco browser reminder worker

The root Loco application reads `MEDTRACKER_WEB_PUSH_PRIVATE_KEY` and
`MEDTRACKER_WEB_PUSH_SUBJECT`. Keep the VAPID signing key consistent across web
and worker replicas. Apply the root PostgreSQL migrations before starting the
worker; `notification_events` and `notification_deliveries` hold reminder intent
and each device's durable outcome. The root `config/production.yaml` schedules
`browser-reminders` once per minute, using the existing PostgreSQL queue. Run
`task worker LOCO_ENV=production` in the approved worker environment. Do not
start a second scheduler for the same deployment by accident.

The scheduler rediscovers due and missed reminders from the preceding hour and
low-stock crossings from the preceding 24 hours. It also queues accounts with
pending or sending deliveries, so retry drain does not require rediscovery of
the original intent. The worker uses its current clock and checks access,
subscription ownership, preferences and dose or stock relevance before each
device attempt. Due and missed deliveries expire one hour after their intended
notification time; the missed time includes the 30-minute grace period.
Low-stock deliveries expire 24 hours after threshold crossing.

Household delivery settings belong to the recipient Account. Notifications about
another Person require an explicit subscription for that Account, household and
Person. New subscriptions default to off for adults, children and dependent
adults alike. Current participation makes a Person eligible for selection; it
does not establish guardianship or consent. Delivery rechecks both current
participation and the saved opt-in.

The scheduler uses the restricted application database role. A narrow database
capability exposes only eligible Account and household IDs, in pages of at most
100; the scheduler enqueues each page before fetching another. This is an
application-wide ID-discovery capability, not an isolated worker credential or
permission to read clinical records or profile fields without a recipient.

Every poll also performs terminal cleanup through a separate atomic capability,
including inactive households and former recipients. It changes at most 100
outstanding deliveries per poll and does not scan terminal delivery history.
Revoked participation, disabled or closed accounts, security lockouts and
inactive households invalidate pending deliveries. Sending claims at least
120 seconds old according to the database clock become `uncertain`, even when
the former recipient can no longer authenticate. Cleanup and its actorless
audit commit together. It cannot send, create retries or revive terminal rows.
Completion still requires recipient access: if access disappears while a send
is in flight, the known provider result cannot be recorded through that path
and the claim becomes `uncertain` on cleanup. Restoring access does not resend it.

Browser payloads identify their owning account. Before displaying a reminder or
opening its destination, the service worker fetches the authenticated profile
push status and requires that account to match. This check has a three-second
timeout and fails closed while signed out or offline. An account transition
waits for any in-flight display, closes notifications already shown by this
browser, and pauses new display while sign-in or sign-out completes. The page
checks the active worker protocol and upgrades an old worker before starting
the transition. The upgrade has a ten-second deadline and activates across
open tabs, including when another worker is already waiting. Activation closes
retained notifications after the old worker's in-flight events finish and
claims the open tabs. The page then waits up to five seconds for the compatible
worker's acknowledgement. If the worker cannot
confirm cleanup, the authentication form stays on the page with retry guidance.
The transition belongs to the initiating tab, survives a service-worker
restart, and expires after 30 minutes if that tab never completes navigation.
This coordinates browser display only; authenticated session checks and
subscription ownership remain server-side. A transition lasting longer than
30 minutes can resume browser delivery, still subject to the current-account
check.

Each device has its own row. `accepted` means the push service accepted the
request; it does not prove the browser displayed it. `pending` with outcome
`confirmed_rejection` is retryable after 1, 5, 15 and 30 minutes, subject to
`Retry-After` and the deadline; no device is sent more than five times.
`uncertain` means transport acceptance could not be determined and is never
retried automatically. `expired` removes only the affected subscription;
`permanent` is terminal. `invalid` records that a deadline, eligibility check
or retry limit prevented another attempt. A sending claim stranded for at least
120 seconds becomes `uncertain` on the next drain. Do not reset terminal
rows or manually replay an uncertain send: the provider may already have
accepted it.

For safe inspection, use an approved read-only database session and aggregate
only status, bounded outcome and age. For example:

```sql
SELECT status, outcome, count(*)
FROM notification_deliveries
GROUP BY status, outcome
ORDER BY status, outcome;
```

Check the minute scheduler and worker are running, queue health, the count of
old `sending` rows, and whether pending rows are past `next_attempt_at`.
Restart the existing worker under the normal process supervisor to recover a
stalled queue; the next drain classifies old claims without replaying them.
Investigate persistent `pending` or `sending` counts before changing the
schedule. Never print subscription endpoints, keys, credentials, payloads,
event metadata, person names or medication text in logs or support output.

The same values can be stored in encrypted credentials:

```yaml
vapid:
  subject: notifications@example.com
  public_key: public-key
  private_key: private-key
```

## Apple push notifications

APNs uses token-based authentication with an Apple signing key.

| Variable | Required | Purpose |
| --- | --- | --- |
| `APNS_BUNDLE_ID` | yes | iOS application bundle identifier and APNs topic. |
| `APNS_TEAM_ID` | yes | Apple developer team identifier. |
| `APNS_KEY_ID` | yes | Identifier of the APNs signing key. |
| `APNS_PRIVATE_KEY` | yes | ES256 private key for the APNs signing token. |
| `APNS_SANDBOX` | no | Uses the APNs sandbox when true. Production is the default. |
| `APNS_HOST` | no | Overrides the APNs endpoint. Leave unset for normal Apple endpoints. |

An environment value for `APNS_PRIVATE_KEY` can contain escaped `\n`
characters. MedTracker converts them to line breaks before loading the key.

Encrypted credentials use the same fields under `apns`:

```yaml
apns:
  bundle_id: io.example.medtracker
  team_id: apple-team-id
  key_id: apple-key-id
  private_key: |
    -----BEGIN PRIVATE KEY-----
    private-key-data
    -----END PRIVATE KEY-----
  sandbox: false
```

## Firebase Cloud Messaging

FCM uses the HTTP v1 message endpoint.

| Variable | Required | Purpose |
| --- | --- | --- |
| `FCM_PROJECT_ID` | yes | Firebase or Google Cloud project identifier. |
| `FCM_BEARER_TOKEN` | yes | OAuth bearer token used for FCM HTTP v1 requests. |

The bearer token is short-lived provider credential material. The deployment
must refresh and replace it before expiry. Restart or reload the application
after changing it.

Encrypted credentials can store the current values under `fcm`:

```yaml
fcm:
  project_id: your-project-id
  bearer_token: current-bearer-token
```

## Channel behavior

MedTracker sends notifications only to subscriptions and device tokens that
belong to the target account. An unconfigured native provider is skipped. An
expired web or native token is removed after the provider reports that it is no
longer registered.

Provider failures do not expose provider response bodies in application logs.
Operational events use bounded channel, provider, stage, and outcome values.

## Verification

Physical browser receipt, operating-system tray behaviour, iPhone Home Screen
delivery and release-image trust/static-link checks remain tracked in
[issue #2559](https://github.com/damacus/med-tracker/issues/2559). Local browser
automation and provider acceptance do not close those gates.

Use test accounts and devices that contain no real health data.

1. Configure one channel at a time.
2. Restart the web and worker processes.
3. Create or refresh the test subscription or device token.
4. Send a test notification from the profile notification settings.
5. Confirm delivery and review the privacy-safe provider outcome event.
6. Remove the test subscription or token when the check is complete.

Do not use a canary environment's provider credentials, applications, or
device tokens in production.
