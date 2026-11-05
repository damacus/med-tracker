# Roles and importer access audit

Source audit on 5 November 2026. This records implementation evidence and
follow-up recommendations for developers and operators.

## Household roles

An account has a role in each household it joins. Household roles and platform
administration are separate permissions. The following describes the root Rust
application, audited against main commit `9f5f3f373` on 5 November 2026.

| Role or authority | Capabilities and limits |
| --- | --- |
| Household owner | Household administration, including members, invitations, settings and audit. Can remove owner authority subject to the last-active-owner safeguard. Platform owner recovery can appoint an eligible owner only when no usable owner remains. |
| Household administrator | Household administration, including members, invitations, settings and audit. Cannot change owner authority. |
| Household member | View and record care for participating people in the household. Cannot use household administration. |
| Active platform administrator | Platform users, platform settings, platform administrator grants/revocations, owner recovery and support-session operations; also the NHS dm+d importer. Does not automatically become a household member or receive unrestricted care access. |

Membership must be active and not revoked; requests also recheck account and
household state. A role in one household does not confer a role in another.
Suspended and revoked memberships are access states, not additional roles.

Platform support access is a separate, time-limited, audited journey requiring
household-owner approval and activation. It provides a support read view rather
than ordinary household membership. A platform administrator record alone does
not authorise browsing every household's clinical records.

## Access to a person's records

Current Rust care checks allow active household participants to view and record
for participating people. Managing a person's care requires self authority or
effective guardian declaration authority. Being a household owner or
administrator does not by itself grant authority to manage another person's
care. Dedicated minor-guardian recovery is available to household managers when
its eligibility checks pass.

Person types (`adult`, `minor`, `dependent_adult`), capacity, care relationships,
guardian declarations and household participation are distinct from account
roles. A relationship label alone does not grant management authority. The older
`view`, `record` and `manage` person-grant description in historical documents
does not describe the current Rust care access checks.

## Administration pages and NHS dm+d access

Household administration is at `/households/{slug}/admin`. Its navigation covers
overview, members, care relationships, invitations, audit and household settings.
These pages require household owner or administrator authority.

Platform administration starts at `/platform/users`; its navigation covers users,
settings, support access and owner recovery. The security page shows a platform
administration link only for an active platform administrator.

The NHS dm+d importer is at `/admin/nhs-dmd`. Both opening the page and uploading
a release require a signed-in account with an **active platform administrator**
record. A household owner or administrator without that separate record is
denied. The current household dashboard and platform navigation do not contain
an importer link; open its URL directly. See [NHS dm+d integration](../nhs-dmd-integration.md#loco-migration-checkout).

### Synthetic seed and reported usability gaps

The standard development/test seed creates an active household `administrator`
membership, but creates no platform administrator record. The canary fixtures
also create household administrators and a member without a platform
administrator. Neither seed therefore supplies a platform administrator for
exploring the importer. The standard seed requires an empty application database
and is limited to development/test. The canary seeder has separate database and
control guards. Do not reset a populated database to obtain another role.

During local synthetic-app exploration on 5 November 2026, the user reported
that the sample account could open household administration but the importer
displayed bare text: “You are not authorized to perform this action”. Source
inspection confirms that the importer returns a plain-text HTTP 403 through the
shared error helper when the platform administrator record is absent. This audit
did not rerun that browser journey or inspect the account's current local roles.

The user expects a seeded administrator and expects admin-page access to include
the importer. That is desired behaviour, not the current permission contract.
Recommended follow-up: provide an explicit development/test platform-admin
fixture for platform exploration, label household and platform administration
clearly, expose an importer link to authorised accounts, and render denied
access using the app's layout and accessible error styling. Keep the existing
platform-only importer checks unless a separate permission-design change is
approved. This documentation change grants no roles and changes no application
behaviour.

### Audit sources

- [Cedar role policies](https://github.com/damacus/med-tracker/blob/9f5f3f3736092bb5b228878478268d0e12310fed/src/models/authorization/policies.cedar), [care access checks](https://github.com/damacus/med-tracker/blob/9f5f3f3736092bb5b228878478268d0e12310fed/src/models/access.rs) and [membership changes](https://github.com/damacus/med-tracker/blob/9f5f3f3736092bb5b228878478268d0e12310fed/src/models/care/administration/memberships.rs).
- [Importer authorisation](https://github.com/damacus/med-tracker/blob/9f5f3f3736092bb5b228878478268d0e12310fed/src/controllers/nhs_dmd.rs), [plain-text error response](https://github.com/damacus/med-tracker/blob/9f5f3f3736092bb5b228878478268d0e12310fed/src/controllers/medications.rs) and [platform support](https://github.com/damacus/med-tracker/blob/9f5f3f3736092bb5b228878478268d0e12310fed/src/models/platform/support.rs).
- [Standard seed](https://github.com/damacus/med-tracker/blob/9f5f3f3736092bb5b228878478268d0e12310fed/src/models/seed.rs), [sample membership](https://github.com/damacus/med-tracker/blob/9f5f3f3736092bb5b228878478268d0e12310fed/src/fixtures/household_memberships.yaml) and [canary fixtures](https://github.com/damacus/med-tracker/blob/9f5f3f3736092bb5b228878478268d0e12310fed/src/models/canary/fixtures.rs).
