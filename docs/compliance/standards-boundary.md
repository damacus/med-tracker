# Standards Boundary

## Status

Internal learning and compliance preparation.

This document explains which standard is being prepared first and which
standards are deferred.

## DCB0129

DCB0129 is the supplier/manufacturer side. For MedTracker, it means the project
must understand and document clinical hazards introduced by the software, the
controls that reduce those hazards, the evidence that those controls work, and
the residual gaps that remain.

This is the first implementation target because MedTracker is the software
project that can document its own intended use, hazards, safety controls, tests,
known gaps, release review process, and incident process.

## DCB0160

DCB0160 is the deployment/use side. A care organisation deploying MedTracker
would need to manage clinical risk in its own local setting.

MedTracker can later provide DCB0160 support material, such as known hazards,
deployment guidance, training prompts, configuration warnings, and residual-risk
notes. MedTracker cannot complete DCB0160 for an adopter because local workflows,
users, training, clinical governance, and rollout decisions belong to that
organisation.

## ISO 13485

ISO 13485 is a medical-device quality management system standard. It covers how
an organisation controls design, development, production, suppliers,
documentation, nonconformities, corrective actions, and lifecycle quality for
medical devices.

ISO 13485 is deferred for this pass because MedTracker is not currently claiming
medical-device status or operating under a formal medical-device QMS.

## SaMD

SaMD means software with its own medical purpose. The current MedTracker
intended-use statement is limited to medication tracking, reminders, rule-based
safeguards, and audit history.

If MedTracker starts recommending diagnosis, treatment, dose changes,
contraindication decisions, or clinical decisions, the SaMD boundary must be
reassessed before implementation.

## Current Implementation Order

1. DCB0129 internal foundation.
2. DCB0160 adopter support pack.
3. SaMD classification review if intended use changes.
4. ISO 13485 or other formal QMS work only if the product and governance model
   require it.

## Source Links

- NHS England Digital clinical risk management standards: <https://digital.nhs.uk/services/clinical-safety/clinical-risk-management-standards>
- DCB0129 page: <https://digital.nhs.uk/data-and-information/information-standards/governance/latest-activity/standards-and-collections/dcb0129-clinical-risk-management-its-application-in-the-manufacture-of-health-it-systems>
- DCB0160 page: <https://digital.nhs.uk/data-and-information/information-standards/governance/latest-activity/standards-and-collections/dcb0160-clinical-risk-management-its-application-in-the-deployment-and-use-of-health-it-systems>
- IMDRF SaMD key definitions: <https://www.imdrf.org/documents/software-medical-device-samd-key-definitions>
- ISO 13485:2016 page: <https://www.iso.org/standard/59752.html>
