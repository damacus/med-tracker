# Manage People and Household Access

MedTracker keeps health records separate from sign-in access. A person can
have a medication record without having an account.

## People and accounts

A **person** is someone whose health and medication are tracked. Their profile
contains their date of birth, medication plans, dose history, and health
events.

An **account** lets someone sign in. An account joins a household through a
household membership. It can also be linked to a person when the account holder
manages their own medication.

A child or dependent adult usually has a person profile without a separate
sign-in.

## Person types

Choose the type that describes the person's support needs:

- **Adult:** an adult who can manage their own medication.
- **Minor:** a child who needs a parent or carer's support.
- **Dependent adult:** an adult who needs someone else to manage their
  medication.

Minors and dependent adults cannot be marked as having capacity to manage
their own medication. Assign at least one parent or carer before saving a
person who does not have capacity.

## Household roles

Your account can have a different role in each household.

| Role | What you can do |
| --- | --- |
| Member | View household care records and record doses and health events. |
| Administrator | Everything a member can do, plus manage household members, invitations, settings and audit records. |
| Owner | Everything an administrator can do, plus remove owner access while keeping at least one active owner. |

Your membership must be active. Access to one household does not give you access
to another.

## Managing a person's care

You can manage your own care. Managing someone else's care requires a current
guardian declaration; being an owner or administrator does not make you their
guardian. A care relationship label alone does not give that permission.

Household owners and administrators can use the guardian recovery journey for
an eligible minor who has no effective guardian.

## Platform administrators

Platform administrators manage the installation's users and settings, recover
households without a usable owner, and import the NHS medicine catalogue.
Platform administration is separate from household roles.

Platform administrators need the household owner's approval for temporary
support access to household care records. Platform status alone does not give
them unrestricted access to everyone's care.

If you have platform access, open **Platform administration** from your account
security page. Household administration remains available to household owners
and administrators.

To import an NHS dm+d release, sign in as a platform administrator and open
`/admin/nhs-dmd` directly; there is currently no importer link in the
administration menus. Household administration access alone is not enough.
See [NHS dm+d import instructions](nhs-dmd-integration.md#loco-migration-checkout).

## Add a person

1. Open **People** and select **New Person**.
2. Enter the person's name and date of birth.
3. Choose the person type.
4. Confirm whether the person has capacity to manage their own medication.
5. Assign a parent or carer when the person needs support.
6. Select **Create Person**.

## Invite someone to the household

A household owner or administrator can invite an account and choose its
household role. Membership grants household-wide viewing and recording for
participating people, so invite only people who should have that access.

Review membership, participation and guardian authority when a care relationship
changes. Use individual accounts so actions remain attributable.

## Continue setup

- [Add their first medication](families/adding-first-medicine.md)
- [Record their first dose](families/taking-first-dose.md)
