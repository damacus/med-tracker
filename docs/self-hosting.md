# Self-Hosting

Self-hosting MedTracker is for operators who are comfortable running web
applications, databases, background jobs, secrets, backups, and upgrades.

For local development commands, use the [Technical Quick Start](quick-start.md).
This page is about running MedTracker for real users.

## Deployment options

MedTracker can run anywhere that supports a Rails application with PostgreSQL,
Redis, and background workers.

Common approaches:

1. **Docker Compose** for a small single-host deployment.
2. **Kubernetes** for a managed cluster, preferably with GitOps.
3. **Platform as a Service** if it can run a Rails web process, worker process,
   PostgreSQL, Redis, and persistent secrets.

See [Deployment](deployment.md) for repository-specific compose files and
operational commands.

## Required services

A production-style MedTracker deployment needs:

1. The Rails web application.
2. A background worker process.
3. PostgreSQL 18.
4. Redis.
5. A configured mail provider for invitations, verification, and account flows.
6. Durable secret management for Rails credentials and environment variables.
7. Backups for the database and any persistent uploaded files.

Use TLS at the edge and set the public application URL to the address your users
will actually open.

## Authentication and first access

Do not rely on development seed users for a self-hosted service.

Bootstrap the first administrator using the deployment mechanism for your
environment, then invite the family members, carers, or support users who should
have access.

For Kubernetes deployments, use the
[Kubernetes User Seeding Runbook](kubernetes-user-seeding.md). For OIDC or
passkeys, use:

- [OAuth/OIDC Setup](oidc-setup.md)
- [Passkey Setup](passkey-setup.md)
- [Two-Factor Authentication](two-factor-authentication.md)

After access is ready, send users the real MedTracker URL and their sign-in
instructions. Their next step is [Add your first medicine](families/adding-first-medicine.md).

## Medicine scanning

Barcode-driven medicine setup depends on the configured medicine lookup data and
services available to your deployment.

For UK dm+d-backed lookup, configure NHS dm+d credentials and imports using:

- [NHS dm+d Integration](nhs-dmd-integration.md)
- [Kubernetes NHS dm+d Release Import](kubernetes-nhs-dmd-import.md)

If lookup is unavailable, users can still enter medication details manually, but
the family onboarding docs assume scanning is the easiest path when configured.

## Operations checklist

Before inviting real users:

1. Confirm the app URL, TLS, mail delivery, and authentication flow.
2. Run database migrations for the deployed release.
3. Create or invite the first administrator.
4. Verify medicine search and barcode scanning in the target environment.
5. Confirm backup and restore procedures.
6. Confirm application logs do not expose sensitive household or medication data.
7. Document who maintains the deployment and how users request access or support.

## User handoff

Once the system is live, family-facing documentation starts here:

- [Add your first medicine](families/adding-first-medicine.md)
- [Record a dose](families/taking-first-dose.md)
- [Top up a medicine by scanning](families/topping-up-medicine.md)
