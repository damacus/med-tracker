# Zitadel Local Testing Guide

This guide covers setting up [Zitadel](https://zitadel.com/) as a local OIDC
provider for testing MedTracker's OpenID Connect authentication.

## Prerequisites

- Docker and Docker Compose
- MedTracker development environment running

## Quick Start

### 1. Start Zitadel

Add a Zitadel service to your local Docker Compose or run standalone:

```bash
docker run -d \
  --name zitadel \
  -p 8080:8080 \
  ghcr.io/zitadel/zitadel:latest start-from-init \
  --masterkey "MasterkeyNeedsToHave32Characters" \
  --tlsMode disabled
```

### 2. Access Zitadel Console

Open `http://localhost:8080` and log in with the default admin credentials:

- **Username**: `zitadel-admin@zitadel.localhost`
- **Password**: `Password1!`

### 3. Create an OIDC Application

1. Navigate to **Projects** and create a new project (e.g., "MedTracker")
2. Add a new **Application** of type **Web**
3. Set **Authentication Method** to **Code** (authorization code flow)
4. Add redirect URI: `http://localhost:3000/auth/oidc/callback`
5. Add post-logout redirect URI: `http://localhost:3000`
6. Save and note the **Client ID** and **Client Secret**

### 4. Create a Test User

1. Navigate to **Users**
2. Create a new user with a verified email address
3. Set a password for the user

### 5. Configure MedTracker

Set environment variables for MedTracker:

```fish
set -x OIDC_ISSUER_URL "http://localhost:8080"
set -x OIDC_CLIENT_ID "your-zitadel-client-id"
set -x OIDC_CLIENT_SECRET "your-zitadel-client-secret"
set -x OIDC_PROVIDER_NAME "Zitadel"
```

### 6. Start MedTracker

```bash
task dev:up
```

### 7. Test the Flow

1. Visit `http://localhost:3000/login`
2. Click **Continue with Zitadel**
3. Log in with the test user created in step 4
4. Verify redirect to MedTracker dashboard

## Verify Discovery Endpoint

Confirm Zitadel's OIDC discovery is accessible:

```bash
curl -s http://localhost:8080/.well-known/openid-configuration | jq .
```

Expected fields include:

- `issuer`: `http://localhost:8080`
- `authorization_endpoint`
- `token_endpoint`
- `jwks_uri`
- `userinfo_endpoint`

## Troubleshooting

### Zitadel container won't start

Ensure port 8080 is not in use:

```bash
lsof -i :8080
```

### "Redirect URI mismatch"

Verify the redirect URI in Zitadel matches exactly:
`http://localhost:3000/auth/oidc/callback`

### Token validation fails

Zitadel uses HTTP locally (no TLS). MedTracker allows HTTP for localhost
issuer URLs automatically.

## Cleanup

```bash
docker stop zitadel
docker rm zitadel
```

## Related Documentation

- [OIDC Setup Guide](oidc-setup.md)
- [Zitadel Documentation](https://zitadel.com/docs)
