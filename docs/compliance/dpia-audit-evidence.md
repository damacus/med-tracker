# DPIA Addendum: Audit Evidence

This addendum supplies technical evidence for a deployment Data Protection
Impact Assessment (DPIA). The deploying organisation must complete and approve
its own DPIA.

## Processing

The audit envelope processes identifiers, roles, authorization decisions,
request context, IP addresses, affected-record identifiers, and redacted event
metadata. Native exports can contain historical source payloads and require the
same or stronger protection as the underlying health data.

## Purpose and necessity

The purposes are patient-safety investigation, security monitoring, access accountability, incident response, support-access oversight, records management, and evidence of control operation. Ordinary HTML page views are excluded to reduce unnecessary collection.

## Risks and controls

| Risk | Control | Residual decision |
|---|---|---|
| Tokens or credentials enter logs | Shared redaction, opaque session references, credential type/record IDs only | Review new event metadata before release |
| Household evidence leaks | Per-household chains, tenant-filtered admin view, dedicated verifier/exporter roles | Restrict exported files and WORM read access |
| History is changed or deleted | Immutable runtime grants, chained hashes, signed checkpoints, Object Lock copies | Database-owner activity needs external oversight |
| Excessive retention | Versioned schedule and `retain_until`; no indefinite claim | Records manager/DPO approve production policy |
| Existing history is overstated | Distinct `legacy-baseline` epoch and explicit limitation | Preserve baseline wording in reports |
| Export creates another health-data copy | Signed manifest and audited export to a controlled output path | Deployment must define transfer and deletion handling |

## Required approval

The deploying organisation must document its lawful basis and access roles. It
must also document the retention schedule, WORM provider and region,
international transfers, data-subject handling, processor terms, incident
contacts, and records-manager and Data Protection Officer approval before
production use.
