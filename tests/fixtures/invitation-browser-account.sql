INSERT INTO public.people(id,account_id,household_id,name,date_of_birth,person_type,has_capacity,created_at,updated_at)
VALUES(73005,71002,72002,'Synthetic invited adult','1990-04-12',0,true,now(),now());
INSERT INTO public.users(id,person_id,email_address,password_digest,created_at,updated_at)
VALUES(77003,73005,'foreign@example.test',crypt('password',gen_salt('bf',4)),now(),now());
INSERT INTO public.household_memberships(id,account_id,household_id,person_id,role,status,joined_at,created_at,updated_at)
VALUES(74003,71002,72002,73005,'owner','active',now(),now(),now());
INSERT INTO public.person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,created_at,updated_at)
VALUES(78007,72002,74003,73005,'manage','self',now(),now());
SELECT setval(pg_get_serial_sequence('public.people','id'),(SELECT max(id) FROM public.people));
