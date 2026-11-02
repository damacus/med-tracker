ALTER TABLE public.legacy_schedule_prescriptions ALTER CONSTRAINT legacy_frozen_source_closure DEFERRABLE INITIALLY DEFERRED;
ALTER TABLE public.legacy_schedule_prescriptions OWNER TO med_tracker_owner;
ALTER TABLE public.legacy_schedule_prescriptions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.legacy_schedule_prescriptions FORCE ROW LEVEL SECURITY;
REVOKE ALL ON public.legacy_schedule_prescriptions FROM PUBLIC,med_tracker_app;
GRANT SELECT,INSERT ON public.legacy_schedule_prescriptions TO med_tracker_app;
GRANT UPDATE(adopted_person_schedule_id) ON public.legacy_schedule_prescriptions TO med_tracker_app;
CREATE POLICY legacy_prescription_capture ON public.legacy_schedule_prescriptions FOR INSERT TO med_tracker_app
WITH CHECK (med_tracker.sharing_household_actor(household_id,true));
CREATE POLICY legacy_prescription_clinical_read ON public.legacy_schedule_prescriptions FOR SELECT TO med_tracker_app
USING (med_tracker.sharing_clinical_authority(person_id));
CREATE POLICY legacy_prescription_adopt ON public.legacy_schedule_prescriptions FOR UPDATE TO med_tracker_app
USING (med_tracker.sharing_clinical_authority(person_id)) WITH CHECK (med_tracker.sharing_clinical_authority(person_id));

CREATE FUNCTION med_tracker.legacy_adoption_actor(p_subject bigint) RETURNS boolean
LANGUAGE sql VOLATILE SET search_path=pg_catalog AS $body$
 SELECT med_tracker.sharing_clinical_authority(p_subject) AND
 ((current_setting('med_tracker.legacy_adoption_declaration_id',true)='self'
   AND EXISTS(SELECT 1 FROM public.accounts actor WHERE actor.id=med_tracker.current_account_id()
    AND actor.person_id=p_subject AND actor.active AND actor.status=2))
 OR med_tracker.sharing_guardian(p_subject)::text=current_setting('med_tracker.legacy_adoption_declaration_id',true))
$body$;
ALTER FUNCTION med_tracker.legacy_adoption_actor(bigint) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.legacy_adoption_actor(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION med_tracker.legacy_adoption_actor(bigint) TO med_tracker_app;

CREATE FUNCTION med_tracker.legacy_prescription_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $body$
DECLARE source public.schedules%ROWTYPE; stock public.medications%ROWTYPE; pause record;
BEGIN
 IF TG_OP='UPDATE' THEN
  IF OLD.adopted_person_schedule_id IS NOT NULL OR NEW.adopted_person_schedule_id IS NULL
   OR (to_jsonb(NEW)-'adopted_person_schedule_id') IS DISTINCT FROM (to_jsonb(OLD)-'adopted_person_schedule_id')
   OR OLD.schedule_id::text IS DISTINCT FROM current_setting('med_tracker.legacy_adoption_source_id',true)
   OR NOT COALESCE(med_tracker.legacy_adoption_actor(OLD.person_id),false)
   OR NOT EXISTS(SELECT 1 FROM public.shared_schedule_states state WHERE state.id=NEW.adopted_person_schedule_id
    AND state.person_id=OLD.person_id AND state.clinical_revision=1
    AND state.origin_time_zone=COALESCE(OLD.origin_time_zone,current_setting('med_tracker.legacy_adoption_zone',true))
    AND state.latest_taken_at IS NULL AND state.last_closed_pause_instance_id IS NULL
    AND (state.active,state.retired_at,state.medicine_name,state.dmd_system,state.dmd_code,state.dmd_concept_class,
     state.stock_unit,state.stock_dose_default,state.stock_default_known,state.dose_amount,state.dose_unit,state.dose_cycle,
     state.frequency,state.start_date,state.end_date,state.max_daily_doses,state.min_hours_between_doses,
     state.schedule_type,state.schedule_config,state.pause_started_at,state.pause_instance_id)
    IS NOT DISTINCT FROM
    (OLD.active,OLD.retired_at,OLD.medicine_name,OLD.dmd_system,OLD.dmd_code,OLD.dmd_concept_class,
     OLD.stock_unit,OLD.stock_dose_default,OLD.stock_default_known,OLD.dose_amount,OLD.dose_unit,OLD.dose_cycle,
     OLD.frequency,OLD.start_date,OLD.end_date,OLD.max_daily_doses,OLD.min_hours_between_doses,
     OLD.schedule_type,OLD.schedule_config,OLD.pause_started_at,OLD.pause_instance_id)) THEN
   RAISE EXCEPTION 'Exact protected prescription adoption required' USING ERRCODE='42501';
  END IF;
  RETURN NEW;
 END IF;
 IF NEW.adopted_person_schedule_id IS NOT NULL OR NEW.source_closed_active
  OR NEW.source_ended_at<>NEW.frozen_at
  OR NEW.captured_by_account_id<>med_tracker.current_account_id()
  OR NEW.captured_by_membership_id<>med_tracker.current_membership_id()
  OR NOT med_tracker.sharing_household_actor(NEW.household_id,true) THEN
  RAISE EXCEPTION 'Current local management capture required' USING ERRCODE='42501';
 END IF;
 SELECT * INTO source FROM public.schedules WHERE id=NEW.schedule_id AND household_id=NEW.household_id FOR UPDATE;
 IF NOT FOUND OR source.person_schedule_id IS NOT NULL OR source.execution_ended_at IS NOT NULL
  OR (NEW.source_portable_id,NEW.person_id,NEW.household_id,NEW.origin_stock_id,NEW.active,NEW.retired_at,
   NEW.dose_amount,NEW.dose_unit,NEW.dose_cycle,NEW.frequency,NEW.start_date,NEW.end_date,
   NEW.max_daily_doses,NEW.min_hours_between_doses,NEW.schedule_type,NEW.schedule_config,NEW.origin_time_zone,NEW.source_created_at)
  IS DISTINCT FROM
  (source.portable_id,source.person_id,source.household_id,source.medication_id,source.active,source.retired_at,
   source.dose_amount,source.dose_unit,source.dose_cycle,source.frequency,source.start_date,source.end_date,
   source.max_daily_doses,source.min_hours_between_doses,source.schedule_type,source.schedule_config,source.origin_time_zone,source.created_at) THEN
  RAISE EXCEPTION 'Exact live legacy source required' USING ERRCODE='42501';
 END IF;
 SELECT * INTO stock FROM public.medications WHERE id=source.medication_id AND household_id=NEW.household_id FOR SHARE;
 IF NOT FOUND OR (NEW.origin_stock_portable_id,NEW.medicine_name,NEW.dmd_system,NEW.dmd_code,NEW.dmd_concept_class,NEW.stock_unit)
  IS DISTINCT FROM (stock.portable_id,stock.name,stock.dmd_system,stock.dmd_code,stock.dmd_concept_class,stock.dose_unit)
  OR NOT ((NEW.stock_default_known AND NEW.stock_dose_default::double precision IS NOT DISTINCT FROM stock.dose_amount)
   OR (NOT NEW.stock_default_known AND NEW.stock_dose_default IS NULL AND stock.dose_amount IS NOT NULL)) THEN
  RAISE EXCEPTION 'Exact local prescription stock required' USING ERRCODE='42501';
 END IF;
 IF (SELECT count(*) FROM public.medication_pause_periods WHERE schedule_id=source.id AND household_id=source.household_id AND ended_at IS NULL)>1 THEN
  RAISE EXCEPTION 'Ambiguous current pause' USING ERRCODE='42501';
 END IF;
 SELECT portable_id,started_at INTO pause FROM public.medication_pause_periods
 WHERE schedule_id=source.id AND household_id=source.household_id AND ended_at IS NULL FOR SHARE;
 IF source.active THEN
  IF NEW.pause_instance_id IS NOT NULL OR NEW.pause_started_at IS NOT NULL THEN
   RAISE EXCEPTION 'Only actual current pause status may be captured' USING ERRCODE='42501';
  END IF;
 ELSIF (NEW.pause_instance_id,NEW.pause_started_at) IS DISTINCT FROM (pause.portable_id,pause.started_at) THEN
  RAISE EXCEPTION 'Exact current pause status required' USING ERRCODE='42501';
 END IF;
 RETURN NEW;
END $body$;
ALTER FUNCTION med_tracker.legacy_prescription_guard() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.legacy_prescription_guard() FROM PUBLIC,med_tracker_app;
CREATE TRIGGER legacy_prescription_guard BEFORE INSERT OR UPDATE ON public.legacy_schedule_prescriptions
FOR EACH ROW EXECUTE FUNCTION med_tracker.legacy_prescription_guard();

CREATE FUNCTION med_tracker.legacy_terminal_source_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $body$
BEGIN
 IF OLD.execution_ended_at IS NOT NULL AND (TG_OP='DELETE' OR NEW IS DISTINCT FROM OLD) THEN
  RAISE EXCEPTION 'Ended execution is immutable' USING ERRCODE='42501';
 END IF;
 IF TG_OP='DELETE' THEN RETURN OLD; END IF;
 IF current_user='med_tracker_app' AND OLD.person_schedule_id IS NULL AND OLD.execution_ended_at IS NULL
  AND NOT COALESCE(med_tracker.sharing_clinical_authority(OLD.person_id),false)
  AND (to_jsonb(NEW)-'updated_at') IS DISTINCT FROM (to_jsonb(OLD)-'updated_at') THEN
  IF NEW.execution_ended_at IS NULL OR NEW.active
   OR (to_jsonb(NEW)-ARRAY['execution_ended_at','active','updated_at']) IS DISTINCT FROM (to_jsonb(OLD)-ARRAY['execution_ended_at','active','updated_at']) THEN
   RAISE EXCEPTION 'Clinical authority or exact terminal closure required' USING ERRCODE='42501';
  END IF;
 END IF;
 RETURN NEW;
END $body$;
ALTER FUNCTION med_tracker.legacy_terminal_source_guard() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.legacy_terminal_source_guard() FROM PUBLIC,med_tracker_app;
CREATE TRIGGER legacy_terminal_source_guard BEFORE UPDATE OR DELETE ON public.schedules
FOR EACH ROW EXECUTE FUNCTION med_tracker.legacy_terminal_source_guard();

CREATE POLICY legacy_prescription_canonical_insert ON public.person_schedules FOR INSERT TO med_tracker_app
WITH CHECK (med_tracker.legacy_adoption_actor(person_id) AND EXISTS(
 SELECT 1 FROM public.legacy_schedule_prescriptions frozen WHERE frozen.schedule_id=origin_schedule_id
 AND frozen.person_id=person_schedules.person_id AND frozen.household_id=origin_household_id));
CREATE FUNCTION med_tracker.legacy_canonical_insert_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $body$
DECLARE frozen public.legacy_schedule_prescriptions%ROWTYPE;
BEGIN
 SELECT * INTO frozen FROM public.legacy_schedule_prescriptions WHERE schedule_id=NEW.origin_schedule_id;
 IF NOT FOUND THEN
  IF NULLIF(current_setting('med_tracker.legacy_adoption_source_id',true),'') IS NOT NULL THEN
   RAISE EXCEPTION 'Exact protected prescription required' USING ERRCODE='42501';
  END IF;
  RETURN NEW;
 END IF;
 IF frozen.adopted_person_schedule_id IS NOT NULL OR NEW.clinical_revision<>1
  OR frozen.schedule_id::text IS DISTINCT FROM current_setting('med_tracker.legacy_adoption_source_id',true)
  OR NOT COALESCE(med_tracker.legacy_adoption_actor(frozen.person_id),false)
  OR NEW.notes IS NOT NULL OR NEW.pause_reason IS NOT NULL OR NEW.pause_note IS NOT NULL
  OR (NEW.person_id,NEW.origin_household_id,NEW.origin_schedule_id,NEW.origin_stock_id,NEW.origin_stock_portable_id,
   NEW.medicine_name,NEW.dmd_system,NEW.dmd_code,NEW.dmd_concept_class,NEW.stock_unit,NEW.stock_dose_default,NEW.stock_default_known,
   NEW.active,NEW.retired_at,NEW.dose_amount,NEW.dose_unit,NEW.dose_cycle,NEW.frequency,NEW.start_date,NEW.end_date,
   NEW.max_daily_doses,NEW.min_hours_between_doses,NEW.schedule_type,NEW.schedule_config,NEW.pause_started_at,NEW.pause_instance_id,NEW.created_at)
  IS DISTINCT FROM
  (frozen.person_id,frozen.household_id,frozen.schedule_id,frozen.origin_stock_id,frozen.origin_stock_portable_id,
   frozen.medicine_name,frozen.dmd_system,frozen.dmd_code,frozen.dmd_concept_class,frozen.stock_unit,frozen.stock_dose_default,frozen.stock_default_known,
   frozen.active,frozen.retired_at,frozen.dose_amount,frozen.dose_unit,frozen.dose_cycle,frozen.frequency,frozen.start_date,frozen.end_date,
   frozen.max_daily_doses,frozen.min_hours_between_doses,frozen.schedule_type,frozen.schedule_config,frozen.pause_started_at,frozen.pause_instance_id,frozen.source_created_at)
  OR NEW.origin_time_zone IS DISTINCT FROM COALESCE(frozen.origin_time_zone,current_setting('med_tracker.legacy_adoption_zone',true))
  OR NEW.timezone_provenance IS DISTINCT FROM (CASE WHEN frozen.origin_time_zone IS NULL THEN 'adopted_application_zone' ELSE 'preserved_origin_zone' END) THEN
  RAISE EXCEPTION 'Frozen prescription must be adopted exactly' USING ERRCODE='42501';
 END IF;
 RETURN NEW;
END $body$;
ALTER FUNCTION med_tracker.legacy_canonical_insert_guard() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.legacy_canonical_insert_guard() FROM PUBLIC,med_tracker_app;
CREATE TRIGGER legacy_canonical_insert_guard BEFORE INSERT ON public.person_schedules
FOR EACH ROW EXECUTE FUNCTION med_tracker.legacy_canonical_insert_guard();
