INSERT INTO public.people(id, clinical_name, person_type, has_capacity, created_at, updated_at)
VALUES (73400, 'Managed adult', 0, true, now(), now()),
       (73401, 'Another adult', 0, true, now(), now()),
       (73402, 'Revoked adult', 0, true, now(), now()),
       (73403, 'Dependent adult', 2, false, now(), now());
INSERT INTO public.household_memberships(id, person_id, household_id, role, status, revoked_at, joined_at, created_at, updated_at)
VALUES (74400, 73400, 72001, 'member', 'active', NULL, now(), now(), now()),
       (74401, 73401, 72001, 'member', 'active', NULL, now(), now(), now()),
       (74402, 73402, 72001, 'member', 'revoked', now(), now(), now(), now()),
       (74403, 73403, 72001, 'member', 'active', NULL, now(), now(), now());
