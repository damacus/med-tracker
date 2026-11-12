# Household AI suggestions

AI medication suggestions are off for every household until an owner or
administrator explicitly enables **Allow external AI medication suggestions**
in Household Settings. A paid plan or an imported household does not enable
this permission. The instance must also enable and configure the AI feature,
and the household must have an eligible plan.

Enabling suggestions allows submitted medication details to be sent to the
instance's configured external AI provider. Suggestions also fetch guidance
from the existing trusted public sources. Review the resulting draft before
saving medication or dose information.

An owner or administrator can withdraw permission by clearing the control and
saving. A send already in progress finishes before withdrawal succeeds. After
the withdrawal has been saved, no later suggestion send can start, including
another provider call in an existing request. Data already transmitted cannot
be recalled.

The API exposes `ai_suggestions_enabled` in household administration settings.
Only an explicit boolean changes the value; omitting it preserves the current
decision. Changes use the existing manager authority and atomic household audit.

Each outbound step obtains a fresh tenant transaction and a shared lock on the
household row, then rechecks membership, consent and feature eligibility after
any lock wait. The lock remains held through the send and is released before
unrelated parsing. Settings updates obtain a conflicting exclusive lock and
recheck manager authority before committing the decision and audit together.
This database coordination applies across application processes.
