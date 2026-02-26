# Carer Onboarding: First Dose

This user story walks a new carer from invitation to recording their first
medication administration safely.

## Scenario

You are invited to help manage medicines for a dependent person and need to
record the first dose correctly.

## Step 1: Accept invitation and sign in

1. Open the invitation link before it expires.
2. Complete account setup or sign in.
3. Confirm you can access the dashboard.

Expected result:

- Your account is active.
- You can see people assigned to you as a carer.

## Step 2: Open the dependent person record

1. Navigate to the person you support.
2. Review active medications:
   - schedules
   - person medications (non-schedule)

Expected result:

- You can see medication details and the current allowed administration state.

## Step 3: Check safety constraints before administering

For the target medication, confirm:

- maximum daily dose (`max_daily_doses`)
- minimum time gap (`min_hours_between_doses`)
- last recorded administration time (if any)

Expected result:

- The UI allows dose recording only when constraints are satisfied.

## Step 4: Record the first dose

1. Choose the correct medication entry.
2. Record administration at the current time (or clinically correct time).
3. Submit the dose entry.

Expected result:

- A new `MedicationTake` is created from exactly one source:
  - schedule, or
  - person medication

## Step 5: Verify history and auditability

After recording:

1. Confirm the administration appears in medication history.
2. Confirm timestamps are correct.
3. If you have admin access, verify an audit record exists.

Expected result:

- Dose is visible in history.
- Audit trail reflects the change.

## Common failure cases

### Too soon since last dose

Symptom: system blocks submission due to `min_hours_between_doses`.

Action: wait until next eligible time.

### Daily limit reached

Symptom: system blocks submission due to `max_daily_doses`.

Action: do not administer additional dose; follow clinical policy/escalation.

### Wrong medication source selected

Symptom: invalid or mismatched dose entry path.

Action: choose the correct medication entry and retry.

## Completion criteria

Onboarding is complete when the carer can:

- sign in successfully,
- access assigned person records,
- record a valid first dose,
- verify the dose appears in history.
