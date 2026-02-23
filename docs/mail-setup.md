# Mail Setup

This guide documents MedTracker email delivery configuration.

It is based on the email-delivery work introduced in PR #518.
Where exact implementation details are not yet confirmed in this branch,
placeholders are included so the final values can be filled in quickly.

## What mail is used for

- Invitation emails sent from the admin invite flow
- Account/security emails triggered by Rodauth

## Environment variables

> Fill in any placeholders marked `TODO(PR518)` once PR #518 is finalized.

| Variable              | Required           | Purpose                                                            | Example                          |
|-----------------------|--------------------|--------------------------------------------------------------------|----------------------------------|
| `APP_URL`             | Yes                | Base app URL used for generated links/origins                      | `https://medtracker.example.com` |
| `SMTP_ADDRESS`        | No (recommended)   | SMTP host (defaults to `localhost`)                                | `smtp.mailgun.org`               |
| `SMTP_PORT`           | No (recommended)   | SMTP port (defaults to `587`)                                      | `587`                            |
| `SMTP_USER_NAME`      | Provider-dependent | SMTP username/login                                                | `postmaster@mg.example.com`      |
| `SMTP_PASSWORD`       | Yes                | SMTP password or API key                                           | `<secret>`                       |
| `SMTP_DOMAIN`         | Yes                | HELO/EHLO domain                                                   | `example.com`                    |
| `SMTP_AUTHENTICATION` | No                 | SMTP auth mode (`plain`, `login`, `cram_md5`), defaults to `plain` | `plain`                          |
| `SMTP_STARTTLS`       | No                 | Enable STARTTLS (`true`/`false`), defaults to `true`               | `true`                           |
| `MAIL_FROM`           | TODO(PR518)        | Default sender address for app mailers                             | `no-reply@example.com`           |

## Local development

By default, development does not raise delivery errors.
See `config/environments/development.rb` for current behavior.

Suggested local `.env` values:

```dotenv
APP_URL=http://localhost:3000
SMTP_ADDRESS=localhost
SMTP_PORT=1025
SMTP_USER_NAME=
SMTP_PASSWORD=
SMTP_DOMAIN=localhost
SMTP_AUTHENTICATION=plain
SMTP_STARTTLS=true
# TODO(PR518): Add any new mail vars introduced by PR #518
```

## Production setup

Current production environment config includes Action Mailer placeholders in
`config/environments/production.rb`. Ensure PR #518 wiring matches the final
environment variable names.

### Example secret values (Kubernetes)

The existing Kubernetes seeding runbook already uses these SMTP variables:

```yaml
APP_URL: https://app.yourdomain.example
SMTP_ADDRESS: smtp.example.com
SMTP_PORT: "587"
SMTP_USER_NAME: <smtp-user>
SMTP_PASSWORD: <smtp-password>
SMTP_DOMAIN: yourdomain.example
# TODO(PR518): add any additional SMTP/TLS/from-address keys
```

## Verification checklist

1. Confirm all required env vars are present in the runtime environment.
2. Send an invitation from `/admin/invitations`.
3. Confirm a job is enqueued and delivered.
4. Verify links in the email use the correct host from `APP_URL`/mailer config.
5. Review logs for SMTP authentication/TLS errors.

## Troubleshooting

| Symptom                               | Likely cause                            | Action                                                     |
|---------------------------------------|-----------------------------------------|------------------------------------------------------------|
| Invite created but email not received | SMTP auth/host/port mismatch            | Validate `SMTP_*` credentials and connectivity             |
| Links in email point to wrong host    | `APP_URL` or mailer host misconfigured  | Set correct public app URL                                 |
| Mail silently not sent in development | Delivery errors disabled in development | Temporarily enable `raise_delivery_errors` while debugging |
| Sender address is incorrect           | `MAIL_FROM` not wired or unset          | TODO(PR518): finalize sender env wiring                    |

## Finalization notes for PR #518

Replace these placeholders after reviewing merged code:

- `TODO(PR518): confirm sender/from-address configuration path`
