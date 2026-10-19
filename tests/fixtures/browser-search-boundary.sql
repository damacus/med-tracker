INSERT INTO public.people(id,name,person_type,has_capacity,created_at,updated_at)
SELECT 73100+n,'Needle person ' || lpad(n::text,2,'0'),0,true,now(),now()
FROM generate_series(1,14) AS people(n);
INSERT INTO public.household_memberships(id,household_id,person_id,role,status,revoked_at,joined_at,created_at,updated_at) SELECT 74100+n,72001,73100+n,'member',CASE WHEN n=14 THEN 'revoked' ELSE 'active' END,CASE WHEN n=14 THEN now() ELSE NULL END,now(),now(),now() FROM generate_series(1,14) AS members(n);
INSERT INTO public.medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_by_membership_id,created_at,updated_at)
VALUES (80011,72001,79001,'Needle',10,1,'unit',74001,now(),now());
INSERT INTO public.person_medications(id,household_id,person_id,medication_id,active,position,created_at,updated_at)
VALUES (81113,72001,73113,80011,true,13,now(),now()),(81114,72001,73114,80011,true,14,now(),now());
INSERT INTO public.schedules(id,household_id,person_id,medication_id,active,schedule_type,schedule_config,frequency,created_at,updated_at)
VALUES (83113,72001,73113,80011,true,0,'{}','Daily',now(),now()),(83114,72001,73114,80011,true,0,'{}','Daily',now(),now());
