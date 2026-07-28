# Mail Setup

MedTracker sends invitation and account-security email through Action Mailer.

## Production environment variables

| Variable | Required | Purpose | Example |
| --- | --- | --- | --- |
| `APP_URL` | Yes | Public origin used for links in email | `https://medtracker.example.com` |
| `SMTP_ADDRESS` | Yes | SMTP server | `smtp.mailgun.org` |
| `SMTP_PORT` | No | SMTP port; defaults to `587` | `587` |
| `SMTP_USER_NAME` | Provider-dependent | SMTP username | `postmaster@example.com` |
| `SMTP_PASSWORD` | Provider-dependent | SMTP password or API key | `<secret>` |
| `SMTP_AUTHENTICATION` | No | SMTP auth mode; defaults to `plain` | `plain` |
| `SMTP_STARTTLS` | No | Enables automatic STARTTLS; defaults to `true` | `true` |
| `MAILER_FROM` | No | Sender address; defaults to MedTracker's built-in sender | `MedTracker <no-reply@example.com>` |

`APP_URL` must include a scheme and host. MedTracker refuses to boot in
production when it is absent or malformed.

Example:

```dotenv
APP_URL=https://medtracker.example.com
SMTP_ADDRESS=smtp.example.com
SMTP_PORT=587
SMTP_USER_NAME=medtracker
SMTP_PASSWORD=<secret>
SMTP_AUTHENTICATION=plain
SMTP_STARTTLS=true
MAILER_FROM=MedTracker <no-reply@example.com>
```

Store credentials in the deployment's secret manager. Do not commit them.

## Local development

The development stack routes outgoing messages to its Mailpit service
automatically; no local SMTP variables are required. Start MedTracker with:

```fish
task dev:portless
```

Development email links currently use the Rails development mailer origin,
`http://localhost:3000`. Mail delivery can still be inspected in Mailpit, but
those links do not use the Portless origin.

## Kubernetes

Use the same production variable names in Secrets or ExternalSecrets:

```yaml
APP_URL: https://medtracker.example.com
SMTP_ADDRESS: smtp.example.com
SMTP_PORT: "587"
SMTP_USER_NAME: medtracker
SMTP_PASSWORD: <secret>
SMTP_AUTHENTICATION: plain
SMTP_STARTTLS: "true"
MAILER_FROM: MedTracker <no-reply@example.com>
```

See the [Kubernetes user-seeding runbook](kubernetes-user-seeding.md) for
invitation setup.

## Verification

1. Confirm the runtime has `APP_URL` and the required SMTP credentials.
2. Send an invitation from `/admin/invitations`.
3. Confirm the message is delivered.
4. Verify links use the public `APP_URL`.
5. Review application logs for SMTP authentication or TLS errors.

## Troubleshooting

| Symptom | Likely cause | Action |
| --- | --- | --- |
| Invite created but email not received | SMTP credentials, host, or port are incorrect | Validate the `SMTP_*` values and provider connectivity |
| Links use the wrong host | `APP_URL` is incorrect | Set it to the public application origin |
| Sender address is wrong | `MAILER_FROM` is unset or malformed | Set the complete display name and address |
