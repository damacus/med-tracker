BEGIN;
SET LOCAL med_tracker.current_account_id='71001';
SET LOCAL med_tracker.current_household_id='72001';
SET LOCAL med_tracker.current_membership_id='74001';
UPDATE public.medications SET current_supply=2 WHERE id=80001;
UPDATE public.person_medications SET active=false, retired_at=now()-interval '8 days' WHERE id=81001;
INSERT INTO public.schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at)
VALUES (83998,72001,73001,80001,true,current_date-7,current_date+30,2,'tablet',0,'{"times":["09:00"]}',now()-interval '8 days',now());
INSERT INTO public.medication_takes(id,household_id,schedule_id,dose_amount,dose_unit,taken_at,created_at,updated_at)
VALUES (84998,72001,83998,2,'tablet',now(),now(),now());
SELECT med_tracker.append_take_history(84998,80001);
COMMIT;
