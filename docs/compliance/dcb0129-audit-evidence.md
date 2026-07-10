# Clinical Safety Evidence: Audit Ledger

This record supplies technical evidence for the clinical safety case. It does not declare DCB0129 or DCB0160 compliance.

## Hazard contribution

| Hazard | Potential harm | Control/evidence | Remaining dependency |
|---|---|---|---|
| Medication history is altered without detection | Investigation uses false dose/stock history | Synchronous source trigger, per-household hash chain, immutable runtime grants, verifier corruption cases | Database-owner oversight and operational verification |
| Actor or authority cannot be reconstructed | Unsafe action cannot be attributed or reviewed | Versioned envelope records actor, role, permission version, auth method, policy/query, request and support context | Identity-provider and deployment logs must be retained |
| Audit evidence disappears during outage/restore | Safety incident cannot be reconstructed | Transactional outbox, Object Lock copy, signed checkpoints, restore-divergence procedure | WORM backlog alerts and tested recovery |
| Pre-migration evidence is treated as trustworthy | Incorrect historical assurance | `legacy-baseline` epoch and signed baseline limitation | Reviewers must preserve the label in reports |
| Audit failure blocks or silently loses clinical work | Missed medication workflow or missing evidence | Local ledger is synchronous/fail-closed; WORM delivery is asynchronous and monitored | Capacity and failure-mode review before launch |

The Clinical Safety Officer must link these controls to the hazard log, verify operating evidence, and assess new failure modes from storage, signing-key, and verifier dependencies.

NHS England is reviewing DCB0129 and DCB0160. The safety case owner must track the replacement/revised standard and update this evidence: <https://digital.nhs.uk/data-and-information/information-standards/governance/latest-activity/standards-and-collections/review-of-digital-clinical-safety-standards-dcb0129-and-dcb0160>.
