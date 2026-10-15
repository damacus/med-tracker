# User Management

## Overview

This page describes the agreed Rust household model. The architecture is
accepted; complete implementation acceptance is still pending. See the
[delivery plan](plans/rust-household-guardrails-20261015.md) and
[accounts, people and households decision](adrs/0013-accounts-people-and-households.md).

## Accounts and people

An **account** owns sign-in credentials, profile information and preferences.
It links to exactly one **person**, the clinical subject whose care is tracked.
A person can exist without an account, and can belong to several households.
There is no separate user or duplicate profile record.

Accountless people can receive care without obtaining sign-in access. Minors
cannot sign in. Preserve these person types:

| Type | Capacity rule |
| --- | --- |
| Adult | Capacity is recorded explicitly. |
| Minor | `has_capacity` is false. |
| Dependent adult | `has_capacity` is false. |

Guardianship must be explicit. Being a household manager, or appearing in a
carer relationship, does not by itself establish guardian authority.

## Account profile API

`GET /api/v1/auth/profile` reads the authenticated account's profile without
selecting a household. `PATCH` updates its name and preferences together:

```json
{
  "profile": {
    "name": "Example account",
    "time_zone": "Europe/London",
    "gravatar_enabled": false,
    "mobile_shortcuts": ["profile", "dashboard"]
  }
}
```

Omitted fields stay unchanged. Explicit nulls, unknown fields, clinical fields
and supplied account or person IDs are rejected. Successful responses contain
only `account_id`, `name`, `email`, `time_zone`, `gravatar_enabled`,
`mobile_shortcuts` and `avatar_attached`, inside `data`. Email is read-only here;
avatar storage details and clinical records are excluded.

Account-bound mobile OAuth credentials remain valid for this profile after the
last household membership is revoked. Household-bound API credentials retain
their membership restrictions. Personal keys and integration credentials cannot
use this endpoint. Every response uses `Cache-Control: no-store`.

Name and preference updates have attributable account audit records. Validation
or audit failure leaves both unchanged. See the [API contract](api/openapi.v1.yaml)
for request and response schemas.

## Personal history access

The self-history reader is being implemented. It will let an account holder
read their original medication administrations across households, including
after leaving every household. A dependent adult with a legitimate account
retains access to their own records. Original names and administration facts
must remain unchanged when current profiles, medicines or sources change.

Global clinical-history access uses the browser session or account-bound
first-party mobile OAuth. Permission to read account profile fields does not
grant this access. Household-bound credentials retain their household limits;
personal keys and integration credentials cannot gain global history access.
See the [authentication and tenancy decision](adrs/0015-authentication-membership-and-tenancy.md).

The first reader covers original medication administrations. Complete personal
history, browser downloads and shared care still require their own implementation
and acceptance; a passing dose-history test does not establish those journeys.

## Household membership

A household is the tenancy boundary for medication inventory and stock.
Membership belongs to a person. A signed-in account uses its linked person's
current memberships.

| Role | Household access |
| --- | --- |
| Owner | Member care access and household administration; the last active owner cannot be removed. |
| Administrator | Member care access and membership administration within the role's limits. |
| Member | View and record care for current household participants. |

There are no separate intra-household person view, record or manage grants.
Manager-only actions remain subject to their own checks. Platform administration
does not automatically grant household clinical access.

A person can create a household or accept a manager's invitation. Sharing an
existing person's care with another household additionally requires approval
from that adult person or an authorised guardian. An invitation must not be
treated as that approval or as proof of guardianship.

## Active household

Each session or device selects an active household from its current memberships.
Switching one device must not switch another device. The active household
determines the inventory, stock and storage labels shown.

An open form and any queued write stay bound to their original household.
Revoked membership must be rejected at request and mutation boundaries. Late
responses from the previous household must not populate the newly selected
household's screen.

## Storage labels and care continuity

Locations are optional physical storage labels, such as a kitchen cupboard.
They have no memberships or permissions. Medication can be created and used
without a location. Removing a label detaches medication from it while retaining
stock and recorded history; see [record lifecycle](operations/record-lifecycle.md).

The accepted care-continuity design shares current treatments, schedules,
active pauses and latest administration status with current households. It does
not automatically share another household's detailed history or narrative notes.
The person controls additional disclosure and retains personal-history access.
After departure, the old household retains its original events and attribution,
without receiving future updates.

Care sharing, guardian approval and explicit transfer of person-allocated stock
still require complete implementation acceptance. Communal stock never moves
automatically. See the [inventory and personal care decision](adrs/0014-household-inventory-and-personal-care.md)
for these rules and their acceptance requirements.

Only the person or an explicitly authorised guardian may change a shared
treatment plan's medicine, dose, schedule or pause. A household manager may
explicitly match local stock to the prescribed medicine, with an audit record.
The application must never infer that match from medicine names or dose text.
These rules are accepted decisions; their implementation remains unfinished.
