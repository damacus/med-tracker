INSERT INTO public.people(id, name, person_type, has_capacity, created_at, updated_at)
VALUES (73400, 'Managed adult', 0, true, now(), now()),
       (73401, 'View-only adult', 0, true, now(), now()),
       (73402, 'Revoked adult', 0, true, now(), now());
INSERT INTO public.household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74410,72001,73400,'member','active',now(),now(),now()),(74411,72001,73401,'member','active',now(),now(),now()),(74412,72001,73402,'member','active',now(),now(),now());
INSERT INTO public.person_access_grants(id, household_id, household_membership_id, person_id, access_level, relationship_type, revoked_at, created_at, updated_at)
VALUES (78400, 72001, 74001, 73400, 'manage', 'family_member', NULL, now(), now()),
       (78401, 72001, 74001, 73401, 'view', 'family_member', NULL, now(), now()),
       (78402, 72001, 74001, 73402, 'manage', 'family_member', now(), now(), now()),
       (78403, 72001, 74001, 73002, 'manage', 'parent', NULL, now(), now()),
       (78404, 72001, 74001, 73003, 'manage', 'family_member', NULL, now(), now());
INSERT INTO public.household_memberships(id, person_id, household_id, role, joined_at, created_at, updated_at)
VALUES (74400, 73005, 72001, 'member', now(), now(), now());
INSERT INTO public.person_access_grants(id, household_id, household_membership_id, person_id, access_level, relationship_type, created_at, updated_at)
VALUES (78405, 72001, 74400, 73401, 'manage', 'family_member', now(), now());
