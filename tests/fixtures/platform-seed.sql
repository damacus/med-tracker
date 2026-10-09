INSERT INTO public.platform_admins(account_id, status, created_at, updated_at)
VALUES (71001, 'active', now(), now()) ON CONFLICT (account_id) DO UPDATE SET status='active', updated_at=now();

INSERT INTO public.accounts(id, email, password_hash, status, created_at, updated_at)
VALUES (71004, 'platform-target@example.test', crypt('password', gen_salt('bf', 4)), 2, now(), now());
INSERT INTO public.people(id, account_id, household_id, name, date_of_birth, person_type, has_capacity, created_at, updated_at)
VALUES (73006, 71004, 72001, 'Synthetic platform target', current_date - interval '30 years', 0, true, now(), now());
INSERT INTO public.users(id, person_id, email_address, password_digest, active, created_at, updated_at)
VALUES (77006, 73006, 'platform-target@example.test', crypt('password', gen_salt('bf', 4)), true, now(), now());
INSERT INTO public.identity_onboarding(account_id, recovery_saved_at) VALUES (71004, now()) ON CONFLICT DO NOTHING;
INSERT INTO public.household_memberships(id, account_id, household_id, person_id, role, joined_at, created_at, updated_at)
VALUES (74006, 71004, 72001, 73006, 'member', now(), now(), now());

INSERT INTO public.households(id, created_by_account_id, name, slug, timezone, status, lifecycle_state, created_at, updated_at)
VALUES (72601, 71001, 'Recovery browser household', 'recovery-browser', 'Europe/London', 'active', 'active', now(), now());
INSERT INTO public.accounts(id, email, password_hash, status, created_at, updated_at)
VALUES (71601, 'recovery-member@example.test', crypt('password', gen_salt('bf', 4)), 2, now(), now());
INSERT INTO public.people(id, account_id, household_id, name, date_of_birth, person_type, has_capacity, created_at, updated_at)
VALUES (73601, 71601, 72601, 'Recovery browser member', current_date - interval '30 years', 0, true, now(), now());
INSERT INTO public.users(id, person_id, email_address, password_digest, active, created_at, updated_at)
VALUES (77601, 73601, 'recovery-member@example.test', crypt('password', gen_salt('bf', 4)), true, now(), now());
INSERT INTO public.identity_onboarding(account_id, recovery_saved_at) VALUES (71601, now()) ON CONFLICT DO NOTHING;
INSERT INTO public.household_memberships(id, account_id, household_id, person_id, role, joined_at, created_at, updated_at)
VALUES (74601, 71601, 72601, 73601, 'member', now(), now(), now());

INSERT INTO public.households(id, created_by_account_id, name, slug, timezone, status, lifecycle_state, created_at, updated_at)
VALUES (72602, 71001, 'Supported browser household', 'supported-browser', 'Europe/London', 'active', 'active', now(), now());
INSERT INTO public.accounts(id, email, password_hash, status, created_at, updated_at)
VALUES (71605, 'support-owner@example.test', crypt('password', gen_salt('bf', 4)), 2, now(), now());
INSERT INTO public.people(id, account_id, household_id, name, date_of_birth, person_type, has_capacity, created_at, updated_at)
VALUES (73605, 71605, 72602, 'Supported browser owner', current_date - interval '30 years', 0, true, now(), now());
INSERT INTO public.users(id, person_id, email_address, password_digest, active, created_at, updated_at)
VALUES (77605, 73605, 'support-owner@example.test', crypt('password', gen_salt('bf', 4)), true, now(), now());
INSERT INTO public.identity_onboarding(account_id, recovery_saved_at) VALUES (71605, now()) ON CONFLICT DO NOTHING;
INSERT INTO public.household_memberships(id, account_id, household_id, person_id, role, joined_at, created_at, updated_at)
VALUES (74605, 71605, 72602, 73605, 'owner', now(), now(), now());
INSERT INTO public.accounts(id, email, password_hash, status, created_at, updated_at)
VALUES (71611, 'supported-member@example.test', crypt('password', gen_salt('bf', 4)), 2, now(), now());
INSERT INTO public.people(id, account_id, household_id, name, date_of_birth, person_type, has_capacity, created_at, updated_at)
VALUES (73611, 71611, 72602, 'Supported browser member', current_date - interval '30 years', 0, true, now(), now());
INSERT INTO public.users(id, person_id, email_address, password_digest, active, created_at, updated_at)
VALUES (77611, 73611, 'supported-member@example.test', crypt('password', gen_salt('bf', 4)), true, now(), now());
INSERT INTO public.identity_onboarding(account_id, recovery_saved_at) VALUES (71611, now()) ON CONFLICT DO NOTHING;
INSERT INTO public.household_memberships(id, account_id, household_id, person_id, role, joined_at, created_at, updated_at)
VALUES (74611, 71611, 72602, 73611, 'member', now(), now(), now());
INSERT INTO public.locations(id, household_id, name, created_at, updated_at)
VALUES (79601, 72602, 'Supported browser cabinet', now(), now());
INSERT INTO public.medications(id, household_id, location_id, name, current_supply, dose_amount, dose_unit, created_at, updated_at)
VALUES (82601, 72602, 79601, 'Supported tablets', 10, 2, 'tablet', now(), now());
INSERT INTO public.person_medications(id, household_id, person_id, medication_id, dose_amount, dose_unit, max_daily_doses, position, created_at, updated_at)
VALUES (83601, 72602, 73611, 82601, 2, 'tablet', 3, 1, now(), now());
INSERT INTO public.schedules(id, household_id, person_id, medication_id, active, schedule_type, schedule_config, frequency, dose_amount, dose_unit, created_at, updated_at)
VALUES (84601, 72602, 73611, 82601, true, 0, '{"times":["08:00","20:00"]}', 'twice_daily', 2, 'tablet', now(), now());
