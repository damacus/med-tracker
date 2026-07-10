# Audit Evidence Retention Policy

Policy version: `clinical-security-v1`

Status: technical default awaiting deployment-specific records-manager and DPO approval.

## Schedule

The default minimum retention period for clinical/security audit evidence is ten years from the recorded event. Evidence inherits a longer period when the related clinical record, contract, statutory inquiry, litigation requirement, or approved local schedule requires it.

Every ledger envelope stores the policy version and calculated `retain_until`. Later policy changes create a new version; they do not rewrite past decisions.

## End-of-period review

`retain_until` means eligible for review, not automatic deletion. The reviewer must confirm the governing record category, current law/policy, open incidents, inquiries, litigation, complaints, and legal holds. Continued retention needs a reason and review date.

No automated disposal is enabled. A future disposal workflow must produce an immutable manifest identifying the approved scope, policy, approvers, time, and resulting Object Lock/database disposition.

## Legal holds

When a hold is issued:

1. Record its authority, scope, owner, start date, and review date in the organisation's records system.
2. Stop disposal for every matching chain/export.
3. Extend Object Lock retention where required; never shorten existing retention.
4. Record the change/incident reference in the signed evidence manifest.
5. Require records-manager or legal approval to release the hold.

Database-owner changes made to implement a hold require a separate external change record.

Reference: [NHS Records Management Code of Practice 2021](https://transform.england.nhs.uk/media/documents/NHSX_Records_Management_CoP_V7.pdf).
