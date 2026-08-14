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

Use test accounts and devices that contain no real health data.

1. Configure one channel at a time.
2. Restart the web and worker processes.
3. Create or refresh the test subscription or device token.
4. Send a test notification from the profile notification settings.
5. Confirm delivery and review the privacy-safe provider outcome event.
6. Remove the test subscription or token when the check is complete.

Do not use a canary environment's provider credentials, applications, or
device tokens in production.
