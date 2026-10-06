UPDATE public.accounts SET status = 2 WHERE id = 71001;
INSERT INTO public.users(id, person_id, email_address, password_digest, created_at, updated_at)
VALUES (77001, 73001, 'persistence@example.test', crypt('password', gen_salt('bf', 4)), now(), now());
INSERT INTO public.person_access_grants(id, household_id, household_membership_id, person_id, access_level, relationship_type, created_at, updated_at)
VALUES (88001, 72001, 74001, 73001, 'manage', 'self', now(), now());
INSERT INTO public.locations(id, household_id, name, created_at, updated_at)
VALUES (89001, 72001, 'Synthetic stock cabinet', now(), now());
INSERT INTO public.medications(id, household_id, location_id, name, current_supply, dose_amount, dose_unit, created_by_membership_id, created_at, updated_at)
VALUES (90001, 72001, 89001, 'Synthetic stock tablets', 10, 2, 'tablet', 74001, now(), now());
