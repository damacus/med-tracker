# DPIA Addendum: Audit Evidence

This addendum is an input to a deployment DPIA, not a completed DPIA or legal approval.

## Processing

The audit envelope processes identifiers, roles, authorization decisions, request context, IP addresses, affected-record identifiers, and redacted event metadata. Native exports can contain historical source payloads and therefore require the same or stronger protection as the underlying health data.

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
| Export creates another PHI copy | Signed manifest, audited export, controlled output path | Deployment must define transfer and deletion handling |

## Required approval

The deploying controller must document lawful basis, access roles, retention schedule, WORM provider/region, international transfers, data-subject handling, processor terms, incident contacts, and records-manager/DPO approval before production use.
