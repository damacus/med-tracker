UPDATE public.accounts SET status = 2 WHERE id = 71001;
INSERT INTO public.users(id, person_id, email_address, password_digest, created_at, updated_at)
VALUES (77001, 73001, 'persistence@example.test', crypt('password', gen_salt('bf', 4)), now(), now());
INSERT INTO public.person_access_grants(id, household_id, household_membership_id, person_id, access_level, relationship_type, created_at, updated_at)
VALUES (78001, 72001, 74001, 73001, 'manage', 'self', now(), now());
INSERT INTO public.locations(id, household_id, name, created_at, updated_at)
VALUES (79001, 72001, 'Synthetic cabinet', now(), now());
INSERT INTO public.medications(id, household_id, location_id, name, current_supply, dose_amount, dose_unit, created_at, updated_at)
VALUES (80001, 72001, 79001, 'Synthetic tablets', 10, 2, 'tablet', now(), now());
INSERT INTO public.person_medications(id, household_id, person_id, medication_id, dose_amount, dose_unit, position, created_at, updated_at)
VALUES (81001, 72001, 73001, 80001, 2, 'tablet', 0, now(), now());
