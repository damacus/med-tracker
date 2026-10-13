INSERT INTO public.accounts(id,email,status,password_hash,created_at,updated_at)
VALUES(71002,'foreign@example.test',2,crypt('password',gen_salt('bf',4)),now(),now());
INSERT INTO public.identity_onboarding(account_id,recovery_saved_at)
VALUES(71001,now()),(71002,now());
INSERT INTO public.households(id,created_by_account_id,name,slug,timezone,created_at,updated_at)
VALUES(72002,71002,'Synthetic foreign household','foreign-fixture','Europe/London',now(),now());
INSERT INTO public.locations(id,household_id,name,created_at,updated_at)
VALUES(79002,72002,'Synthetic foreign cabinet',now(),now());
INSERT INTO public.medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at)
VALUES(80002,72002,79002,'Synthetic foreign medicine',10,2,'tablet',now(),now());
