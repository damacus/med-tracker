# MedTracker Documentation

MedTracker is an open-source, self-hosted medication tracker for individuals,
families, and carers. It supports household medication schedules, dose
recording, stock tracking, reminders, and auditable history.

!!! important "MedTracker is in beta"
    MedTracker should supplement, not replace, your existing medication routine.
    Do not depend on it for clinical decisions, emergency information, or your
    sole medication reminders.

## Try the self-hosted beta

Start with the [self-hosting guide](self-hosting.md) for private local
evaluation. Technically confident operators planning a reachable installation
should also read the [deployment guide](deployment.md).

We welcome feedback through GitHub:

- [Discussions](https://github.com/damacus/med-tracker/discussions) for
  questions, early impressions, and self-hosting help
- [Guided issue forms](https://github.com/damacus/med-tracker/issues/new/choose)
  for bugs and feature requests
- [Private security reports](https://github.com/damacus/med-tracker/security/advisories/new)
  for suspected vulnerabilities

Never post names, medication details, health information, credentials, tokens,
or unredacted logs publicly.

---

## 🏠 For Families
*These guides are for family members and carers looking after loved ones at home.*

- [**Quick Setup Guide**](families/quick-setup.md): get MedTracker up and running in minutes.
- [**Add your first medicine**](families/adding-first-medicine.md): learn how to add a prescription or a simple over-the-counter medicine.
- [**Record a dose**](families/taking-first-dose.md): follow the steps to safely record when a medicine is taken.
- [Manage your family members](user-management.md): set up profiles for the people you support.

---

## 🛠️ For Developers
*These guides are for those setting up, customizing, or contributing to the MedTracker codebase.*

- [**Technical Quick Start**](quick-start.md): run the full stack with Docker.
- [Testing Guide](testing.md): run the RSpec and Capybara/Playwright test suites.
- [RubyUI comparison](ruby-ui-comparison.md): compare generated RubyUI files before choosing a manual update.
- [Design & Architecture](design.md): explore the domain model and safety guardrails.
- [Bounded Context Map](adrs/0009-bounded-context-map.md): see present domain ownership and dependency direction in the Rails modular monolith.
- [Proposed record lifecycle](operations/record-lifecycle.md): review the
  unimplemented design for medication, person, and location retirement.
- [Audit & Compliance](audit-trail.md): details on versioning and data history.
- [MCP Integration](mcp.md): set up the hosted MCP server and connect Codex,
  Claude Code, or VS Code to read medication context.
- [Client Tools](api/client-tools.md): install and operate the first-party
  `medtracker` CLI and `medtracker-mcp` stdio server over `/api/v1`.
- [API Contract Conventions](api/README.md): use canonical API addressing,
  stable operation IDs, and audience/resource tags when extending the OpenAPI contract.
- [API Contract](api/openapi.v1.yaml): OpenAPI contract for hosted auth, portable IDs, sync, backups, admin APIs, and FHIR R4 reads.
- [API Versioning Policy](api/versioning.md): classify compatible changes,
  deprecations, breaking changes, and generated-client ownership.
- [Portable Data Format](api/portable-data.md): understand snapshot, export,
  import, and incremental sync record fields.
- [External Integration Architecture](adrs/0010-external-integration-architecture.md): choose `/api/v1`, `/mcp`, or SMART on FHIR by client audience.
- [Production Upload Storage](adrs/0008-production-upload-storage.md): understand optional Disk and S3 modes, topology, and migration boundaries.
- [Upload Storage Backup and Restore](operations/upload-storage-backup-and-restore.md): record and verify backend-aware recovery sets.
- [Home-Ops Portable Storage Handoff](operations/home-ops-portable-storage-handoff.md): apply the deployment inputs and maintenance gates safely.
- [Pre-0.5 database upgrade](pre-0-5-database-upgrade.md): bootstrap existing PostgreSQL databases before the household/RLS cutover.

---

## 🩺 For Clinicians & Advanced Users
*These guides focus on clinical accuracy and deep integrations.*

- [NHS dm+d Integration](nhs-dmd-integration.md): use the UK's medicine dictionary to find accurate names.
- [Kubernetes NHS dm+d Release Import](kubernetes-nhs-dmd-import.md): import dm+d AMPP and GTIN release files in production.
- [UK Regulatory Compliance Plan](uk-regulatory-compliance-plan.md): how MedTracker aligns with health data standards.
- [Audit Trail](audit-trail.md): how we ensure clinical records are safe and traceable.

---

### Need help?
- [Glossary](glossary.md): common terms used in the app.
- [Troubleshooting](quick-start.md#troubleshooting): common technical fixes.
