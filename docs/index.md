# MedTracker Documentation

MedTracker helps carers, families, and clinicians record medication
administration safely with auditability and timing safeguards.

## Start Here

- [Quick Start](quick-start.md): run MedTracker locally with Docker.
- [Carer Onboarding: First Dose](user-onboarding-carer-first-dose.md): first-time
  user flow from invite to recording a dose.
- [Testing](testing.md): run the full suite with project `task` commands.

## Core Guides

- [Deployment](deployment.md): run app and database in development and production.
- [Kubernetes User Seeding](kubernetes-user-seeding.md): bootstrap first admin and seed care-team invites using ConfigMap and Secret/ExternalSecret patterns.
- [User Management](user-management.md): person types, user roles, and carer links.
- [Mail Setup](mail-setup.md): SMTP/environment configuration and delivery verification checklist.
- [OAuth Setup](oauth-setup.md): configure Google OIDC sign-in.
- [Passkey Setup](passkey-setup.md): configure and operate WebAuthn passkeys.
- [Two-Factor Authentication](two-factor-authentication.md): TOTP and recovery flow.

## Integrations

- [NHS dm+d Medicine Search](nhs-dmd-integration.md): search the
  NHS Dictionary of Medicines and Devices, credentials setup, and
  feature gating.

## Architecture

- [Design](design.md): architecture, domain model, and guardrails.
- [Database Relationships](database-relationships.md): current schema relationships.
- [Audit Trail](audit-trail.md): PaperTrail model coverage and access rules.
- [Accessibility](accessibility.md): WCAG guidance used in the UI.
- [Theming](theming.md): styling and design consistency rules.

## Docs Tooling

- Local preview: `task docs:serve`
- Static build: `task docs:build`
- LLM context index: [llms.txt](llms.txt)

Published docs: <https://damacus.github.io/med-tracker/>
