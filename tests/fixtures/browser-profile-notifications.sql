INSERT INTO public.people(id, household_id, name, person_type, has_capacity, created_at, updated_at)
VALUES (73400, 72001, 'Managed adult', 0, true, now(), now()),
       (73401, 72001, 'View-only adult', 0, true, now(), now()),
       (73402, 72001, 'Revoked adult', 0, true, now(), now());
INSERT INTO public.person_access_grants(id, household_id, household_membership_id, person_id, access_level, relationship_type, revoked_at, created_at, updated_at)
VALUES (78400, 72001, 74001, 73400, 'manage', 'family_member', NULL, now(), now()),
       (78401, 72001, 74001, 73401, 'view', 'family_member', NULL, now(), now()),
       (78402, 72001, 74001, 73402, 'manage', 'family_member', now(), now(), now()),
       (78403, 72001, 74001, 73002, 'manage', 'parent', NULL, now(), now()),
       (78404, 72001, 74001, 73003, 'manage', 'family_member', NULL, now(), now());
INSERT INTO public.household_memberships(id, account_id, household_id, role, joined_at, created_at, updated_at)
VALUES (74400, 71002, 72001, 'member', now(), now(), now());
INSERT INTO public.person_access_grants(id, household_id, household_membership_id, person_id, access_level, relationship_type, created_at, updated_at)
VALUES (78405, 72001, 74400, 73401, 'manage', 'family_member', now(), now());
