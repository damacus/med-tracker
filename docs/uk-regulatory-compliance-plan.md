# UK Regulatory Compliance and Open‑Source Deployment Plan

Version: 1.0
Date: 2025-11-12

## Executive summary

This document outlines the compliance strategy for operating MedTracker in the UK health context and as an open-source project. It covers regulatory determination (medical device vs. non-device), mandatory applications/registrations, clinical safety, information governance and privacy, security baseline, accessibility, auditability, interoperability, and an implementation backlog for the repository and deployers.

MedTracker’s compliance posture depends primarily on its intended use. The plan below treats both tracks:

- Non-medical device track (recordkeeping and workflow support only)
- Software as a Medical Device (SaMD) track (if the app performs safety‑critical clinical decision support beyond simple reminders/recording)

## 1. Intended use and scope (decision point)

- Draft intended use statement (to ratify):
  - “MedTracker is a software application that helps carers and patients schedule, record, and review medication administration, enforcing configured rules (e.g., max daily doses, minimum intervals) and providing safety prompts. It does not diagnose conditions or recommend treatments; clinical decisions remain with healthcare professionals.”
- Decision criteria:
  - If the app computes or recommends dosing, contraindications, or interactions as a basis for clinical decisions, classify as SaMD.
  - If it provides reminders, logging, and rule-based checks configured by clinicians without autonomous recommendations, likely non‑medical device administrative software.
- Action:
  - Approve a written intended use statement and freeze scope. If any feature moves it into SaMD, follow the SaMD track below.

## 2. Regulatory pathways (UK)

- Non‑medical device track:
  - UK GDPR and Data Protection Act 2018 compliance
  - NHS Digital Technology Assessment Criteria (DTAC) evidence for NHS procurement
  - Clinical safety DCB0129 (manufacturer) and DCB0160 (deploying organisation) if used in NHS settings
  - NHS Data Security and Protection Toolkit (DSPT) completed by the deploying organisation
- SaMD track (if applicable):
  - Device classification; likely Class I/IIa depending on functionality
  - Quality Management System (QMS) (ISO 13485‑aligned), ISO 14971 risk management, IEC 62304 software lifecycle (proportionate)
  - Technical documentation (“technical file”), clinical evaluation, post‑market surveillance plan
  - MHRA registration and UKCA marking via an Approved Body where required
  - Continue to meet DTAC, DCB0129/0160, DSPT, and UK GDPR obligations

## 3. Required applications and registrations

- ICO registration (data controller): required for organisations deploying MedTracker in production with personal data.
- NHS DSP Toolkit (DSPT): required for NHS bodies and suppliers processing NHS patient data; completed by the deploying organisation.
- DCB0129/0160 safety case approvals: manufacturer (supplier) and deploying care organisation sign‑off with appointed Clinical Safety Officers (CSOs).
- MHRA manufacturer registration and UKCA marking: only if MedTracker is SaMD.
- Cyber Essentials / Cyber Essentials Plus: recommended for supplier trust and some procurement pathways.
- SNOMED CT / TRUD licensing: if using SNOMED CT codes; ensure adopters have appropriate licences. Avoid embedding full SNOMED content in the repo.
- NHS BSA dm+d (Dictionary of Medicines and Devices): verify licensing (typically OGL) and update acknowledgements if dm+d is used.
- DTAC evidence pack: prepare and submit as part of NHS procurement.

## 4. Information governance and privacy (UK GDPR)

- Lawful basis and special category condition selection (likely Art. 6(1)(e) or (f) and Art. 9(2)(h) or (c)/(g) depending on deployer context).
- Data Protection Impact Assessment (DPIA) template and guidance for adopters.
- Records of Processing Activities (ROPA) template.
- Privacy Notice and transparency materials; plain English content design.
- Data Subject Rights procedures: access, rectification, erasure, restriction, portability, objection.
- Children’s data and Age Appropriate Design Code: accommodate minors with high‑privacy defaults.
- Data retention schedules and deletion workflows; configurable policies.
- Data processing agreements and subprocessor disclosures for hosted models.
- International transfers assessment (SCCs/IDTA) if applicable.

## 5. Security baseline (OWASP ASVS L2 target)

- Authentication and session security; RBAC with least privilege; audit authorization rules.
- TLS 1.2+ everywhere; HSTS; secure cookies; CSRF protection.
- Secret management (12‑factor); key rotation; no secrets in source control.
- Encryption at rest for databases and backups; KMS‑managed keys.
- Logging: structured, redacted; no secrets/health data in logs; centralised log management.
- Vulnerability management: SCA (bundler‑audit), SAST (CodeQL), DAST (optional), dependency updates (Dependabot/Renovate).
- Supply chain: SBOM (CycloneDX), signature verification, reproducible builds.
- Backups, disaster recovery, RPO/RTO objectives; tested restores.
- Monitoring, alerting, and incident response runbooks with 72‑hour breach notification procedure.

## 6. Clinical safety and human factors (DCB0129/0160)

- Appoint a Clinical Safety Officer (CSO) for manufacturer and for each deploying organisation.
- Create a Safety Case and Hazard Log; use ISO 14971 risk management.
- Human factors: implement the “five rights” prompts, clear error states, and confirmation flows.
- Safety controls already in product scope (examples): max daily doses, minimum hours between doses; ensure these have tests and are traceable in the hazard log.
- Labeling and limitations: disclaimers for non‑emergency use; do not replace professional judgement.
- Post‑market surveillance plan: collect field feedback, near‑misses, and safety incidents.

## 7. Accessibility and inclusivity

- Target WCAG 2.2 AA; run automated checks and manual audits.
- Screen reader compatibility, keyboard navigation, focus management, colour contrast, error messaging.
- Plain language; support for timezones and internationalisation of units (mg/mL).

## 8. Auditability and accountability

- Persistent audit trails for create/update/delete and key access events (who/what/when/where/IP/UA), with immutable retention.
- Avoid logging clinical data in access logs; store audit metadata separately from PHI where possible.
- Provide export of audit records for investigations and regulator requests.
- Tamper‑evidence (append‑only storage or cryptographic sealing) for high‑assurance deployments.

## 9. Interoperability (optional roadmap)

- Use NHS dm+d identifiers for medicines where possible.
- Consider FHIR resources (Medication, MedicationRequest, MedicationStatement, MedicationAdministration) for integration.
- If using SNOMED CT, ensure licence compliance and avoid redistributing restricted content.

## 10. Open‑source governance and licensing

- Licence decision: Apache‑2.0 (chosen to maximize adoption and avoid legal barriers to use).
- Alternative (not chosen): AGPL‑3.0 for stronger network copyleft.
- Project governance: MAINTAINERS.md, CODE_OF_CONDUCT.md, CONTRIBUTING.md, SECURITY.md (vulnerability disclosure and embargo policy).
- Release engineering: changelog, SemVer, signed tags, SBOMs and checksums for releases.
- Compliance kit for adopters: docs/compliance/ with DPIA template, ROPA template, DCB0129/0160 starter packs, DTAC mapping, DSPT guidance.

## 11. Repository implementation backlog (product changes)

- RBAC authorization review and tests per Authorization Completion Plan.
- Audit trail hardening: adopt a proven auditing solution or equivalent access logging; define retention policy and export.
- Data retention settings: configurable schedules, purge jobs, user‑initiated deletion workflows.
- Security controls in code and CI: Brakeman, bundler‑audit, CodeQL, dependency update automation.
- Accessibility: WCAG 2.2 AA audit and fixes; add system tests where reasonable.
- Privacy by design: logging redaction, metrics vs. PHI separation, error reporting configuration.
- Operational: health checks, backup/restore scripts, monitoring hooks, incident response runbooks.

## 12. Deployment and adopter checklist

- Determine role (controller/processor) and complete ICO registration if controller.
- Complete DSPT (for NHS data) and assemble DTAC evidence pack.
- Appoint CSO and complete local DCB0160 activities; obtain supplier’s DCB0129 evidence.
- Complete DPIA and ROPA; publish Privacy Notice; sign DPAs.
- Implement security baseline: TLS, RBAC, secrets, backups, monitoring, incident response.
- Validate accessibility (WCAG 2.2 AA) and usability with representative users.
- Run vulnerability scans and pen test prior to go‑live; track remediation.

## 13. Milestones and timeline (suggested)

- 0–30 days: Intended use decision; DPIA/ROPA templates; baseline security in CI; audit trail plan; RBAC review; draft DCB0129 skeleton; create compliance kit folder.
- 30–60 days: Accessibility audit and fixes; data retention; incident response; backups/DR; DTAC mapping; DSPT guidance for adopters.
- 60–90 days: Clinical safety evidence drafts complete; monitoring and alerting; SBOM in releases; optional Cyber Essentials submission.
- If SaMD: establish QMS and begin MHRA/UKCA planning in parallel; extend timeline accordingly.

## 14. Documentation to add to repository (next steps)

- SECURITY.md (vuln disclosure and support windows)
- GOVERNANCE/MAINTAINERS.md
- docs/compliance/ with templates: DPIA, ROPA, DCB0129/0160 starter, DTAC mapping, DSPT checklist
- Deployment Guide with compliance checklist for adopters

## References

- UK GDPR and Data Protection Act 2018
- NHS DTAC (Digital Technology Assessment Criteria)
- NHS DSP Toolkit
- DCB0129 (Clinical Risk Management: Manufacturers of Health IT Systems)
- DCB0160 (Clinical Risk Management: Deployment and Use of Health IT Systems)
- ISO 14971, IEC 62304, ISO 13485 (if SaMD)
- NHS BSA dm+d, SNOMED CT licensing and TRUD
