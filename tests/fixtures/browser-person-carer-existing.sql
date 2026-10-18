UPDATE public.person_access_grants SET access_level='manage' WHERE id=78005;
INSERT INTO public.carer_relationships(id,household_id,carer_id,patient_id,relationship_type,active,created_at,updated_at)
VALUES(86005,72001,73004,73002,'parent',true,now(),now());
INSERT INTO public.person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,carer_relationship_id,created_at,updated_at)
VALUES(78008,72001,74002,73002,'manage','parent',86005,now(),now());
