INSERT INTO public.locations(id,household_id,name,created_at,updated_at)
VALUES (79011,72001,'Travel bag',now(),now());
UPDATE public.medications SET current_supply=0,category='Routine' WHERE id=80001;
INSERT INTO public.medications(id,household_id,location_id,name,category,current_supply,dose_unit,created_by_membership_id,created_at,updated_at)
VALUES (80011,72001,79011,'Synthetic spare capsules','As needed',NULL,'capsule',74001,now(),now());
INSERT INTO public.medications(id,household_id,location_id,name,category,current_supply,dose_unit,created_by_membership_id,created_at,updated_at)
VALUES (80021,72001,79011,'Synthetic option tablets','As needed',5,'tablet',74001,now(),now());
INSERT INTO public.dosages(id,household_id,medication_id,amount,unit,current_supply,frequency,default_dose_cycle,default_max_daily_doses,default_min_hours_between_doses,created_at,updated_at)
VALUES (82021,72001,80021,1,'tablet',2,'daily',0,1,0,now(),now()),
       (82022,72001,80021,2,'tablet',3,'daily',0,1,0,now(),now());
INSERT INTO public.medications(id,household_id,location_id,name,category,current_supply,dose_unit,created_by_membership_id,created_at,updated_at)
VALUES (80031,72001,79011,'Synthetic untracked option tablets','As needed',NULL,'tablet',74001,now(),now());
INSERT INTO public.dosages(id,household_id,medication_id,amount,unit,current_supply,frequency,default_dose_cycle,default_max_daily_doses,default_min_hours_between_doses,created_at,updated_at)
VALUES (82031,72001,80031,1,'tablet',NULL,'daily',0,1,0,now(),now());
