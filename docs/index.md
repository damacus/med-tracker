# MedTracker Documentation

MedTracker helps you safely record when someone has taken their medicine.
It's built to keep families, carers, and health professionals on the same page,
ensuring medications are given on time and safely.

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
- [Design & Architecture](design.md): explore the domain model and safety guardrails.
- [Audit & Compliance](audit-trail.md): details on versioning and data history.
- [MCP Integration](mcp.md): set up the hosted MCP server and connect Codex,
  Claude Code, or VS Code to read medication context.
- [Pre-0.5 database upgrade](pre-0-5-database-upgrade.md): bootstrap existing PostgreSQL databases before the household/RLS cutover.

---

## 🩺 For Clinicians & Advanced Users
*These guides focus on clinical accuracy and deep integrations.*

- [NHS dm+d Integration](nhs-dmd-integration.md): use the UK's medicine dictionary to find accurate names.
- [Kubernetes NHS dm+d Release Import](kubernetes-nhs-dmd-import.md): import dm+d AMPP and GTIN release files in production.
- [UK Regulatory Compliance Plan](uk-regulatory-compliance-plan.md): how MedTracker aligns with health data standards.
- [Compliance Foundations](compliance/index.md): internal learning and preparation for clinical safety evidence.
- [DCB0129 Foundations Teaching Page](compliance/learning/dcb0129-foundations.html): visual guide to what we are doing and why.
- [Audit Trail](audit-trail.md): how we ensure clinical records are safe and traceable.

---

### Need help?
- [Glossary](glossary.md): common terms used in the app.
- [Troubleshooting](quick-start.md#troubleshooting): common technical fixes.
