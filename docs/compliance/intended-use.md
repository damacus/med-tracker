# Intended Use

## Status

Internal draft for learning and compliance preparation.

This statement describes the current product boundary. It does not make a public
clinical, regulatory, NHS, medical-device, or certification claim.

## Intended Use Statement

MedTracker is an open-source household medication tracking application. It helps
a household record medicines, schedules, reminders, dose events, stock status,
care relationships, and audit history.

MedTracker may show rule-based safeguards configured from stored medication
schedule data, such as dose timing limits, daily dose limits, stock availability,
and reminder status. These safeguards are intended to support recordkeeping and
safer household routines.

## Current Users

The current real-world use is limited to the maintainer's household. Future
users are not assumed by this document.

## What MedTracker Does

- Stores medication, schedule, person, household, dose, stock, and care
  relationship records.
- Records when a medication dose is taken.
- Shows reminders and missed-dose indicators based on configured schedules.
- Applies rule-based safeguards such as maximum daily doses and minimum time
  between doses.
- Keeps audit history for safety-relevant records.
- Supports household roles and access control.

## What MedTracker Does Not Do

- MedTracker does not diagnose conditions.
- MedTracker does not recommend treatment.
- MedTracker does not recommend autonomous dose changes.
- MedTracker does not decide whether a medicine is clinically appropriate.
- MedTracker does not replace clinical judgement.
- MedTracker does not provide emergency care advice.
- MedTracker does not claim medical-device status in this draft.
- MedTracker does not claim NHS approval, clinical approval, or certification.

## Current SaMD Boundary

On the current intended-use statement, MedTracker is being treated as health IT
for medication tracking, reminders, rule-based safeguards, and audit history.

If future features recommend diagnosis, treatment, dose changes,
contraindication decisions, medication interactions, or clinical decision
outputs that users rely on for treatment, this statement must be reviewed before
the feature is implemented.

## Change Control Rule

Any feature that changes the intended-use boundary must update this document and
the DCB0129 evidence map in the same pull request.
