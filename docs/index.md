# MedTracker Documentation

MedTracker helps families and carers keep a clear record of everyday medicine:
what is in the house, who it is for, and when it was taken.

Most people reading these docs are joining a MedTracker system that someone
else has already set up. You do not need developer tools or example accounts.
Start with the link and sign-in details your family organiser, carer, or support
team gave you.

---

## For Families and Carers

Use these guides when you are getting started with a real household or care
setup.

- [**Add your first medicine**](families/adding-first-medicine.md): sign in, scan a medicine box, and add it to MedTracker.
- [**Record a dose**](families/taking-first-dose.md): record that someone has taken their medicine.
- [**Top up a medicine by scanning**](families/topping-up-medicine.md): add new stock to a medicine that is already in MedTracker.
- [Manage your family members](user-management.md): set up and manage the people you support.

---

## For People Running MedTracker

Use these guides if you are installing, hosting, or maintaining MedTracker for
someone else.

- [**Technical Quick Start**](quick-start.md): run the full stack with Docker.
- [Self-Hosting Setup](self-hosting.md): run MedTracker on your own computer or server.
- [Testing Guide](testing.md): run the RSpec and Capybara/Playwright test suites.
- [Design & Architecture](design.md): explore the domain model and safety guardrails.
- [Audit & Compliance](audit-trail.md): details on versioning and data history.

---

## Technical Reference

These pages are mainly for maintainers, deployers, and advanced integrations.

- [NHS dm+d Integration](nhs-dmd-integration.md): use the UK's medicine dictionary to find accurate names.
- [Kubernetes NHS dm+d Release Import](kubernetes-nhs-dmd-import.md): import dm+d AMPP and GTIN release files in production.
- [UK Regulatory Compliance Plan](uk-regulatory-compliance-plan.md): how MedTracker aligns with health data standards.
- [Glossary](glossary.md): common terms used in the app.
