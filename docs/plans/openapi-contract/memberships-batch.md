# Household membership administration

Implement listMemberships, updateMembership, replaceMembership and
deleteMembership after the medication families. Luna may prepare tests while
the previous batch completes; production requires initial compiled HTTP RED.

Use the documented payload and response shapes and Rails AccessChange rules.
Owners and administrators can enter these routes, but owner transitions have
additional restrictions: promotion to active owner requires an active platform
administrator; changes away from an active owner require an owner or active
platform administrator. Preserve last-owner and model validation rules.

PATCH and PUT merge supplied role, status and person association fields.
DELETE revokes membership. Scope records to the selected household and reject
foreign person associations, invalid values, unknown fields and forbidden nulls
without changing access. Reauthenticate the actor under the household lock.

Increment permissions_version once for a real access change; unchanged requests
do not increment it. Prove existing sessions and app tokens become unusable
after relevant changes. Preserve household_membership.role_updated versus
household_access.membership_changed audit events, including no_change and
rejected outcomes, without exposing unrelated household data.

Explicit reactivation clears the revocation marker as well as setting active
status. Rust authentication checks both fields; leaving a previous revocation
timestamp would incorrectly deny fresh credentials after reactivation. Keep
the permission-version increment so old credentials remain invalid.

Cover current owner/admin/member and cross-household boundaries, concurrent
last-owner protection, shared idempotency and direct rate-limit wiring. Tests
must use disposable or restored fixture records so credential invalidation
does not contaminate later cases. Sol owns production and shared integration;
independent review and isolated acceptance precede publication and credit.
