# Test Local MedTracker with Zitadel

This guide connects the local MedTracker development stack to a Zitadel
instance. The Zitadel issuer must be reachable from both your browser and the
MedTracker container. A hosted HTTPS issuer is the simplest setup.

## Prerequisites

- A Zitadel instance and project
- Docker, Task, and Portless installed
- Portless trusted once with `portless trust`

## Create the Zitadel application

1. In Zitadel, create or open a project.
2. Add an application of type **Web**.
3. Select the authorization code flow.
4. Add
   `https://med-tracker.localhost/auth/oidc/callback`
   as a redirect URI.
5. Add `https://med-tracker.localhost` as a post-logout redirect URI.
6. Record the issuer URL, client ID, and client secret.

## Start MedTracker

Export the provider and stable Portless URLs before starting the container:

```fish
set -x APP_URL "https://med-tracker.localhost"
set -x OIDC_REDIRECT_URI "https://med-tracker.localhost/auth/oidc/callback"
set -x OIDC_ISSUER_URL "https://your-zitadel-instance.example"
set -x OIDC_CLIENT_ID "your-zitadel-client-id"
set -x OIDC_CLIENT_SECRET "your-zitadel-client-secret"
set -x OIDC_PROVIDER_NAME "Zitadel"
task dev:portless
```

Open <https://med-tracker.localhost/login> and choose **Continue with
Zitadel**.

When invitation-only registration is enabled, use an invited email address.
OIDC does not bypass MedTracker's registration or household-access policy.

## Zitadel claims

MedTracker can synchronise the recognised `doctor` or `nurse` professional
title from Zitadel's `urn:zitadel:iam:org:project:roles` claim. Zitadel roles do
not grant MedTracker household roles or person-level access.

To synchronise a professional title:

1. Create a Zitadel project role called `doctor` or `nurse`.
2. Assign it to the user.
3. Enable **Assert Roles on Authentication** for the application.
4. Sign in again to refresh the claim.

## Troubleshooting

### Redirect URI mismatch

Confirm the provider callback exactly matches:

`https://med-tracker.localhost/auth/oidc/callback`

### Provider discovery fails

The issuer must expose
`.well-known/openid-configuration` and be reachable from the MedTracker
container. Do not use a host-only `localhost` issuer for a containerised
MedTracker instance.

### Portless certificate warning

Run `portless trust`, then restart `task dev:portless`.

## Related documentation

- [OIDC Setup Guide](oidc-setup.md)
- [Zitadel documentation](https://zitadel.com/docs)
