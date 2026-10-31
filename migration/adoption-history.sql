-- SeaORM/SeaQuery cover the table, FK, index and ordinary receipt DML. They do
-- not express PostgreSQL RLS/ACL or fixed validation-trigger DDL. These pure
-- hash/time helpers read no table; runtime has no callable helper privilege.
CREATE FUNCTION med_tracker.adoption_fact_hash(fact public.shared_schedule_adoption_facts) RETURNS bytea
LANGUAGE sql IMMUTABLE SET search_path=pg_catalog AS $body$
 SELECT sha256(convert_to(jsonb_build_array('live-adoption-v1',fact.person_schedule_id,
  fact.source_schedule_id,fact.source_portable_id,fact.person_id,fact.household_id,
  fact.medication_take_id,fact.take_portable_id,fact.original_version_id,fact.client_uuid,
  to_char(fact.taken_at,'YYYY-MM-DD"T"HH24:MI:SS.US'),fact.dose_amount,fact.dose_unit,fact.taken_from_medication_id,
  fact.taken_from_location_id,to_char(fact.take_created_at,'YYYY-MM-DD"T"HH24:MI:SS.US'),
  to_char(fact.take_updated_at,'YYYY-MM-DD"T"HH24:MI:SS.US'))::text,'UTF8'))
$body$;
ALTER FUNCTION med_tracker.adoption_fact_hash(public.shared_schedule_adoption_facts) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.adoption_fact_hash(public.shared_schedule_adoption_facts) FROM PUBLIC,med_tracker_app;

CREATE FUNCTION med_tracker.adoption_audit_time_matches(encoded text,actual timestamp) RETURNS boolean
LANGUAGE plpgsql IMMUTABLE SET search_path=pg_catalog AS $body$
DECLARE decoded timestamp;
BEGIN
 IF encoded IS NULL OR actual IS NULL THEN RETURN encoded IS NULL AND actual IS NULL; END IF;
 IF encoded~'(Z|[+-][0-9]{2}:[0-9]{2})$' THEN decoded:=timezone('UTC',encoded::timestamptz);
 ELSE decoded:=encoded::timestamp; END IF;
 RETURN decoded=actual OR (position('.' in encoded)=0 AND decoded=date_trunc('second',actual));
EXCEPTION WHEN invalid_datetime_format OR datetime_field_overflow THEN RETURN false;
END $body$;
ALTER FUNCTION med_tracker.adoption_audit_time_matches(text,timestamp) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.adoption_audit_time_matches(text,timestamp) FROM PUBLIC,med_tracker_app;

ALTER TABLE public.shared_schedule_adoption_facts OWNER TO med_tracker_owner;
ALTER TABLE public.shared_schedule_adoption_facts ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.shared_schedule_adoption_facts FORCE ROW LEVEL SECURITY;
REVOKE ALL ON public.shared_schedule_adoption_facts FROM PUBLIC,med_tracker_app;
GRANT SELECT,INSERT ON public.shared_schedule_adoption_facts TO med_tracker_app;
CREATE POLICY adoption_fact_local_read ON public.shared_schedule_adoption_facts FOR SELECT TO med_tracker_app
USING (household_id=med_tracker.current_household_id() AND med_tracker.sharing_household_actor(household_id,false));
CREATE POLICY adoption_fact_local_insert ON public.shared_schedule_adoption_facts FOR INSERT TO med_tracker_app
WITH CHECK (household_id=med_tracker.current_household_id() AND med_tracker.sharing_clinical_authority(person_id)
 AND EXISTS(SELECT 1 FROM public.schedules source JOIN public.person_schedules plan ON plan.id=shared_schedule_adoption_facts.person_schedule_id
  WHERE source.id=source_schedule_id AND source.household_id=shared_schedule_adoption_facts.household_id
   AND source.person_id=shared_schedule_adoption_facts.person_id AND source.person_schedule_id IS NULL
   AND source.execution_ended_at IS NULL AND plan.origin_schedule_id=source.id
   AND plan.origin_household_id=source.household_id AND plan.person_id=source.person_id));
CREATE POLICY adoption_fact_fixed_validator ON public.shared_schedule_adoption_facts FOR SELECT TO med_tracker_owner
USING (household_id=med_tracker.current_household_id()
 AND medication_take_id::text=current_setting('med_tracker.sharing_take_id',true));
CREATE TRIGGER adoption_fact_guard BEFORE INSERT ON public.shared_schedule_adoption_facts
FOR EACH ROW EXECUTE FUNCTION med_tracker.shared_administration_guard();

-- Validation only, limited to facts already captured in this table. The owner
-- retains fixture/retention capability; ordinary uncaptured takes are unaffected.
CREATE FUNCTION med_tracker.adopted_take_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $body$
BEGIN
 IF current_user='med_tracker_app' AND (TG_OP='DELETE' OR NEW IS DISTINCT FROM OLD)
  AND EXISTS(SELECT 1 FROM public.shared_schedule_adoption_facts fact WHERE fact.medication_take_id=OLD.id) THEN
  RAISE EXCEPTION 'Captured administration is immutable' USING ERRCODE='42501';
 END IF;
 IF TG_OP='DELETE' THEN RETURN OLD; END IF;
 RETURN NEW;
END $body$;
ALTER FUNCTION med_tracker.adopted_take_immutable() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.adopted_take_immutable() FROM PUBLIC,med_tracker_app;
CREATE TRIGGER adopted_take_immutable BEFORE UPDATE OR DELETE ON public.medication_takes
FOR EACH ROW EXECUTE FUNCTION med_tracker.adopted_take_immutable();
