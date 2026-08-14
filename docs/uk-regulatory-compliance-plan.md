# UK regulatory planning guide

- Status
  Planning guide, not legal or clinical safety approval
- Last reviewed
  2026-08-14

## Purpose

This guide identifies the decisions a UK deployer must make before using
MedTracker with real health data. The repository cannot decide a deployer's
legal role, intended purpose, clinical setting, or procurement duties.

Get advice from a qualified data-protection lead, Clinical Safety Officer, and
medical-device specialist where the deployment needs it.

## Start with intended purpose

Write and approve an intended-purpose statement for the deployed product. Keep
it aligned with the features, claims, user groups, and clinical setting.

The MHRA states that software may be a medical device when its intended purpose
fits the medical-device definition. A record storage or patient-management
system does not become a device only because it stores health information.
Features that diagnose, recommend treatment, or drive a clinical decision need
a fresh classification assessment.

Do not describe MedTracker as a medical device, or as outside medical-device
rules, without that assessment.

## Data protection

Each deployer must identify whether it acts as a controller, processor, or both.
It must record the lawful basis and special-category condition for each use of
health data.

Complete a Data Protection Impact Assessment when the planned processing is
likely to create high risk. The ICO also recommends a DPIA for major projects
that use sensitive data or involve vulnerable people. The assessment must cover
the real hosting model, users, integrations, retention, support access, and
international transfers.

Also prepare:

- a privacy notice and data-subject-rights process;
- records of processing activities;
- processor and subprocessor terms;
- a retention and disposal schedule; and
- incident response and breach assessment procedures.

Check current ICO guidance because UK data-protection law changed after the
Data (Use and Access) Act received Royal Assent in 2025.

## NHS clinical safety and procurement

DCB0129 applies clinical risk management to organisations that develop and
maintain health IT systems for use in health and care. DCB0160 covers the
deployment and use of those systems. NHS England is reviewing both standards,
so a safety case must follow the current release and track later changes.

The supplier and deploying organisation must agree their responsibilities.
They should maintain a hazard log and clinical safety case. Release evidence
and appointed Clinical Safety Officer oversight are also needed where the standards
apply.

NHS procurement may also require:

- Digital Technology Assessment Criteria evidence;
- Data Security and Protection Toolkit evidence;
- accessibility evidence;
- penetration-test and vulnerability-management evidence; and
- interoperability and service-management evidence.

These artefacts do not replace product safety, legal, or security work.

## Medical-device path

If the intended purpose makes MedTracker medical-device software, confirm the
route with an appropriate specialist. The assessment must cover classification,
the quality-management system, risk management, clinical evidence, technical
documentation, post-market surveillance, registration, and the correct market
marking for the target UK nation.

Do not assume a device class or conformity route from this guide. Great Britain
and Northern Ireland can have different requirements, and the rules continue to
change.

## Repository evidence

Use repository documents as inputs to a deployment assessment:

- [Hosted multi-tenant security gate](security/hosted-multi-tenant-hardening-audit.md)
- [Hosted private beta runbook](operations/hosted-private-beta-runbook.md)
- [Audit trail](audit-trail.md)
- [Audit retention policy](compliance/audit-retention-policy.md)
- [DPIA audit evidence](compliance/dpia-audit-evidence.md)
- [DCB0129 audit evidence](compliance/dcb0129-audit-evidence.md)
- [Accessibility guidelines](accessibility.md)
- [Production environment](operations/production-environment.md)

Repository evidence proves only the stated code or operational control. It does
not approve a deployment. The hosted private beta remains closed while its
security gate is `NO-GO`.

## Deployment decision record

Before go-live, record:

- the approved intended purpose and device-classification advice;
- controller and processor roles, lawful basis, and DPIA approval;
- applicable DCB0129 and DCB0160 responsibilities;
- the completed procurement and accessibility evidence;
- the accepted security and recovery evidence; and
- the people authorised to accept residual clinical, privacy, and operational
  risk.

Revisit the decision after material feature, hosting, integration, user, or
regulatory changes.

## Current sources

- [MHRA guidance for software applications](https://www.gov.uk/government/publications/medical-devices-software-applications-apps)
- [MHRA software and AI medical-device guidance](https://www.gov.uk/government/publications/software-and-artificial-intelligence-ai-as-a-medical-device)
- [ICO DPIA guidance](https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/accountability-and-governance/data-protection-impact-assessments-dpias/)
- [NHS England DCB0129 standard](https://digital.nhs.uk/data-and-information/information-standards/governance/latest-activity/standards-and-collections/dcb0129-clinical-risk-management-its-application-in-the-manufacture-of-health-it-systems/)
- [NHS England DTAC](https://www.england.nhs.uk/digital-technology-assessment-criteria-dtac/)
