ALTER TABLE public.frozen_history_invalidations OWNER TO med_tracker_owner;
ALTER TABLE public.frozen_history_invalidations ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.frozen_history_invalidations FORCE ROW LEVEL SECURITY;
REVOKE ALL ON public.frozen_history_invalidations FROM PUBLIC, med_tracker_app;
CREATE POLICY frozen_invalidation_owner_read ON public.frozen_history_invalidations FOR SELECT TO med_tracker_owner
 USING (source_schedule_id::text=current_setting('med_tracker.frozen_proof_source',true));
CREATE POLICY frozen_invalidation_owner_insert ON public.frozen_history_invalidations FOR INSERT TO med_tracker_owner
 WITH CHECK (source_schedule_id::text=current_setting('med_tracker.frozen_proof_source',true));
CREATE POLICY frozen_proof_parent_owner ON public.legacy_schedule_prescriptions FOR SELECT TO med_tracker_owner
 USING (schedule_id::text=current_setting('med_tracker.frozen_proof_source',true));
CREATE POLICY frozen_proof_take_owner ON public.medication_takes FOR SELECT TO med_tracker_owner
 USING (schedule_id::text=current_setting('med_tracker.frozen_proof_source',true));
CREATE POLICY frozen_proof_fact_owner ON public.shared_schedule_adoption_facts FOR SELECT TO med_tracker_owner
 USING (source_schedule_id::text=current_setting('med_tracker.frozen_proof_source',true));
CREATE POLICY frozen_proof_plan_owner ON public.person_schedules FOR SELECT TO med_tracker_owner
 USING (id=current_setting('med_tracker.frozen_proof_plan',true));
CREATE POLICY frozen_proof_state_owner ON public.shared_schedule_states FOR SELECT TO med_tracker_owner
 USING (id=current_setting('med_tracker.frozen_proof_plan',true));
CREATE POLICY frozen_proof_index_owner ON public.shared_schedule_administrations FOR SELECT TO med_tracker_owner
 USING (person_schedule_id=current_setting('med_tracker.frozen_proof_plan',true));

CREATE FUNCTION med_tracker.frozen_read_committed() RETURNS void
LANGUAGE plpgsql VOLATILE SET search_path=pg_catalog AS $body$
BEGIN
 IF current_setting('transaction_isolation') <> 'read committed' THEN
  RAISE EXCEPTION 'Frozen history requires read committed isolation' USING ERRCODE='25001';
 END IF;
END $body$;

-- Internal boolean only. Receipt insertion already validates the earliest audit;
-- this verifies exact immutable receipt coverage and, when bound, safety index.
CREATE FUNCTION med_tracker.frozen_history_proof(p_source bigint, p_index boolean) RETURNS boolean
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=pg_catalog AS $body$
DECLARE parent public.legacy_schedule_prescriptions%ROWTYPE; previous_source text; previous_plan text; valid boolean;
BEGIN
 PERFORM med_tracker.frozen_read_committed();
 previous_source:=current_setting('med_tracker.frozen_proof_source',true);
 previous_plan:=current_setting('med_tracker.frozen_proof_plan',true);
 PERFORM set_config('med_tracker.frozen_proof_source',p_source::text,true);
 SELECT * INTO parent FROM public.legacy_schedule_prescriptions WHERE schedule_id=p_source;
 IF NOT FOUND OR parent.capture_complete IS DISTINCT FROM true THEN
  PERFORM set_config('med_tracker.frozen_proof_source',coalesce(previous_source,''),true);
  RETURN false;
 END IF;
 PERFORM set_config('med_tracker.frozen_proof_plan',coalesce(parent.adopted_person_schedule_id,''),true);
 SELECT NOT EXISTS(SELECT 1 FROM public.frozen_history_invalidations WHERE source_schedule_id=p_source)
  AND NOT EXISTS (
   SELECT 1 FROM public.medication_takes take
   LEFT JOIN public.shared_schedule_adoption_facts fact ON fact.medication_take_id=take.id
   WHERE take.schedule_id=p_source AND take.household_id=parent.household_id
    AND (fact.medication_take_id IS NULL OR fact.person_schedule_id IS NOT NULL
     OR (fact.source_schedule_id,fact.source_portable_id,fact.person_id,fact.household_id,
      fact.take_portable_id,fact.client_uuid,fact.taken_at,fact.dose_amount,fact.dose_unit,
      fact.taken_from_medication_id,fact.taken_from_location_id,fact.take_created_at,fact.take_updated_at)
     IS DISTINCT FROM (parent.schedule_id,parent.source_portable_id,parent.person_id,parent.household_id,
      take.portable_id,take.client_uuid,take.taken_at,take.dose_amount,take.dose_unit,
      take.taken_from_medication_id,take.taken_from_location_id,take.created_at,take.updated_at)
     OR fact.fact_hash IS DISTINCT FROM med_tracker.adoption_fact_hash(fact)))
  AND NOT EXISTS (
   SELECT 1 FROM public.shared_schedule_adoption_facts fact
   LEFT JOIN public.medication_takes take ON take.id=fact.medication_take_id
   WHERE fact.source_schedule_id=p_source AND fact.person_schedule_id IS NULL
    AND (take.id IS NULL OR take.schedule_id IS DISTINCT FROM p_source
     OR take.household_id IS DISTINCT FROM parent.household_id))
  INTO valid;
 IF p_index THEN
  valid:=valid AND parent.adopted_person_schedule_id IS NOT NULL
   AND EXISTS(SELECT 1 FROM public.person_schedules plan WHERE plan.id=parent.adopted_person_schedule_id
    AND plan.origin_schedule_id=p_source AND plan.origin_household_id=parent.household_id AND plan.person_id=parent.person_id)
   AND NOT EXISTS(SELECT 1 FROM public.shared_schedule_adoption_facts fact
    WHERE fact.source_schedule_id=p_source AND fact.person_schedule_id IS NULL
     AND NOT EXISTS(SELECT 1 FROM public.shared_schedule_administrations entry
      WHERE entry.medication_take_id=fact.medication_take_id
       AND entry.person_schedule_id=parent.adopted_person_schedule_id
       AND entry.household_id=fact.household_id AND entry.taken_at=fact.taken_at
       AND entry.occurrence_starts_on IS NULL AND entry.occurrence_position IS NULL))
   AND EXISTS(SELECT 1 FROM public.shared_schedule_states state WHERE state.id=parent.adopted_person_schedule_id
    AND state.person_id=parent.person_id AND (NOT EXISTS(SELECT 1 FROM public.shared_schedule_adoption_facts fact
      WHERE fact.source_schedule_id=p_source AND fact.person_schedule_id IS NULL)
     OR state.latest_taken_at >= (SELECT max(fact.taken_at) FROM public.shared_schedule_adoption_facts fact
      WHERE fact.source_schedule_id=p_source AND fact.person_schedule_id IS NULL)));
 END IF;
 PERFORM set_config('med_tracker.frozen_proof_plan',coalesce(previous_plan,''),true);
 PERFORM set_config('med_tracker.frozen_proof_source',coalesce(previous_source,''),true);
 RETURN coalesce(valid,false);
END $body$;

CREATE FUNCTION public.frozen_adoption_ready(p_source bigint) RETURNS boolean
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=pg_catalog AS $body$
DECLARE previous_source text; parent public.legacy_schedule_prescriptions%ROWTYPE; allowed boolean;
BEGIN
 PERFORM med_tracker.frozen_read_committed();
 previous_source:=current_setting('med_tracker.frozen_proof_source',true);
 PERFORM set_config('med_tracker.frozen_proof_source',p_source::text,true);
 SELECT * INTO parent FROM public.legacy_schedule_prescriptions WHERE schedule_id=p_source;
 allowed:=FOUND AND med_tracker.sharing_clinical_authority(parent.person_id);
 PERFORM set_config('med_tracker.frozen_proof_source',coalesce(previous_source,''),true);
 IF NOT coalesce(allowed,false) THEN RETURN false; END IF;
 RETURN med_tracker.frozen_history_proof(p_source,parent.adopted_person_schedule_id IS NOT NULL);
END $body$;

CREATE FUNCTION med_tracker.frozen_capture_finalize() RETURNS trigger
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=pg_catalog AS $body$
DECLARE previous_source text; previous_plan text; exact_latest boolean;
BEGIN
 PERFORM med_tracker.frozen_read_committed();
 IF NEW.capture_complete IS TRUE AND NOT med_tracker.frozen_history_proof(NEW.schedule_id,NEW.adopted_person_schedule_id IS NOT NULL) THEN
  RAISE EXCEPTION 'Frozen history incomplete' USING ERRCODE='42501';
 END IF;
 IF NEW.adopted_person_schedule_id IS NOT NULL AND NOT med_tracker.frozen_history_proof(NEW.schedule_id,true) THEN
  RAISE EXCEPTION 'Frozen history incomplete' USING ERRCODE='42501';
 END IF;
 IF NEW.adopted_person_schedule_id IS NOT NULL THEN
  previous_source:=current_setting('med_tracker.frozen_proof_source',true);
  previous_plan:=current_setting('med_tracker.frozen_proof_plan',true);
  PERFORM set_config('med_tracker.frozen_proof_source',NEW.schedule_id::text,true);
  PERFORM set_config('med_tracker.frozen_proof_plan',NEW.adopted_person_schedule_id,true);
  SELECT state.latest_taken_at IS NOT DISTINCT FROM
   (SELECT max(fact.taken_at) FROM public.shared_schedule_adoption_facts fact
    WHERE fact.source_schedule_id=NEW.schedule_id AND fact.person_schedule_id IS NULL)
   INTO exact_latest FROM public.shared_schedule_states state WHERE state.id=NEW.adopted_person_schedule_id;
  PERFORM set_config('med_tracker.frozen_proof_plan',coalesce(previous_plan,''),true);
  PERFORM set_config('med_tracker.frozen_proof_source',coalesce(previous_source,''),true);
  IF NOT coalesce(exact_latest,false) THEN
   RAISE EXCEPTION 'Frozen history incomplete' USING ERRCODE='42501';
  END IF;
 END IF;
 RETURN NULL;
END $body$;
CREATE CONSTRAINT TRIGGER frozen_capture_finalize AFTER INSERT OR UPDATE ON public.legacy_schedule_prescriptions
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION med_tracker.frozen_capture_finalize();

CREATE FUNCTION med_tracker.frozen_prevent_incomplete_binding() RETURNS trigger
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=pg_catalog AS $body$
DECLARE source_id bigint; previous_source text; found_parent boolean;
BEGIN
 IF TG_TABLE_NAME='legacy_schedule_prescriptions' THEN
  source_id:=NEW.schedule_id;
 ELSE
  source_id:=NEW.origin_schedule_id;
 END IF;
 IF source_id IS NULL THEN RETURN NEW; END IF;
 PERFORM med_tracker.frozen_read_committed();
 previous_source:=current_setting('med_tracker.frozen_proof_source',true);
 PERFORM set_config('med_tracker.frozen_proof_source',source_id::text,true);
 SELECT EXISTS(SELECT 1 FROM public.legacy_schedule_prescriptions WHERE schedule_id=source_id) INTO found_parent;
 PERFORM set_config('med_tracker.frozen_proof_source',coalesce(previous_source,''),true);
 IF found_parent AND NOT med_tracker.frozen_history_proof(source_id,false) THEN
  RAISE EXCEPTION 'Frozen history incomplete' USING ERRCODE='42501';
 END IF;
 RETURN NEW;
END $body$;
CREATE TRIGGER frozen_binding_proof BEFORE UPDATE OF adopted_person_schedule_id ON public.legacy_schedule_prescriptions
 FOR EACH ROW EXECUTE FUNCTION med_tracker.frozen_prevent_incomplete_binding();
CREATE TRIGGER frozen_canonical_proof BEFORE INSERT ON public.person_schedules
 FOR EACH ROW EXECUTE FUNCTION med_tracker.frozen_prevent_incomplete_binding();

CREATE FUNCTION med_tracker.frozen_canonical_finalize() RETURNS trigger
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=pg_catalog AS $body$
DECLARE parent public.legacy_schedule_prescriptions%ROWTYPE; previous_source text; previous_plan text; exact_latest boolean;
BEGIN
 IF NEW.origin_schedule_id IS NULL THEN RETURN NULL; END IF;
 PERFORM med_tracker.frozen_read_committed();
 previous_source:=current_setting('med_tracker.frozen_proof_source',true);
 previous_plan:=current_setting('med_tracker.frozen_proof_plan',true);
 PERFORM set_config('med_tracker.frozen_proof_source',NEW.origin_schedule_id::text,true);
 SELECT * INTO parent FROM public.legacy_schedule_prescriptions WHERE schedule_id=NEW.origin_schedule_id;
 IF FOUND THEN
  IF parent.adopted_person_schedule_id IS DISTINCT FROM NEW.id
   OR NOT med_tracker.frozen_history_proof(NEW.origin_schedule_id,true) THEN
   RAISE EXCEPTION 'Frozen history incomplete' USING ERRCODE='42501';
  END IF;
  PERFORM set_config('med_tracker.frozen_proof_plan',NEW.id,true);
  SELECT state.latest_taken_at IS NOT DISTINCT FROM
   (SELECT max(fact.taken_at) FROM public.shared_schedule_adoption_facts fact
    WHERE fact.source_schedule_id=NEW.origin_schedule_id AND fact.person_schedule_id IS NULL)
   INTO exact_latest FROM public.shared_schedule_states state WHERE state.id=NEW.id;
  IF NOT coalesce(exact_latest,false) THEN
   RAISE EXCEPTION 'Frozen history incomplete' USING ERRCODE='42501';
  END IF;
 END IF;
 PERFORM set_config('med_tracker.frozen_proof_plan',coalesce(previous_plan,''),true);
 PERFORM set_config('med_tracker.frozen_proof_source',coalesce(previous_source,''),true);
 RETURN NULL;
END $body$;
CREATE CONSTRAINT TRIGGER frozen_canonical_finalize AFTER INSERT ON public.person_schedules
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION med_tracker.frozen_canonical_finalize();

CREATE FUNCTION med_tracker.frozen_invalidate_changed_takes() RETURNS trigger
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=pg_catalog AS $body$
DECLARE changed record; previous_source text;
BEGIN
 previous_source:=current_setting('med_tracker.frozen_proof_source',true);
 FOR changed IN SELECT DISTINCT schedule_id FROM new_takes WHERE schedule_id IS NOT NULL LOOP
  -- Check before inspecting parents, including when the statement began
  -- before a concurrent departure made the parent visible.
  PERFORM med_tracker.frozen_read_committed();
  PERFORM set_config('med_tracker.frozen_proof_source',changed.schedule_id::text,true);
  INSERT INTO public.frozen_history_invalidations(source_schedule_id)
   SELECT changed.schedule_id WHERE EXISTS(SELECT 1 FROM public.legacy_schedule_prescriptions WHERE schedule_id=changed.schedule_id);
 END LOOP;
 PERFORM set_config('med_tracker.frozen_proof_source',coalesce(previous_source,''),true);
 RETURN NULL;
END $body$;
CREATE TRIGGER frozen_invalidate_take_insert AFTER INSERT ON public.medication_takes
 REFERENCING NEW TABLE AS new_takes FOR EACH STATEMENT EXECUTE FUNCTION med_tracker.frozen_invalidate_changed_takes();

CREATE FUNCTION med_tracker.frozen_invalidate_moved_takes() RETURNS trigger
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=pg_catalog AS $body$
DECLARE changed record; previous_source text;
BEGIN
 previous_source:=current_setting('med_tracker.frozen_proof_source',true);
 FOR changed IN
  SELECT source_id AS schedule_id FROM (
   SELECT o.schedule_id AS source_id FROM old_takes o FULL JOIN new_takes n ON n.id=o.id
    WHERE (o.id IS NULL OR n.id IS NULL OR to_jsonb(o) IS DISTINCT FROM to_jsonb(n))
   UNION
   SELECT n.schedule_id AS source_id FROM old_takes o FULL JOIN new_takes n ON n.id=o.id
    WHERE (o.id IS NULL OR n.id IS NULL OR to_jsonb(o) IS DISTINCT FROM to_jsonb(n))
  ) changed_sources WHERE source_id IS NOT NULL LOOP
  PERFORM med_tracker.frozen_read_committed();
  PERFORM set_config('med_tracker.frozen_proof_source',changed.schedule_id::text,true);
  INSERT INTO public.frozen_history_invalidations(source_schedule_id)
   SELECT changed.schedule_id WHERE EXISTS(SELECT 1 FROM public.legacy_schedule_prescriptions WHERE schedule_id=changed.schedule_id);
 END LOOP;
 PERFORM set_config('med_tracker.frozen_proof_source',coalesce(previous_source,''),true);
 RETURN NULL;
END $body$;
CREATE TRIGGER frozen_invalidate_take_update AFTER UPDATE ON public.medication_takes
 REFERENCING OLD TABLE AS old_takes NEW TABLE AS new_takes FOR EACH STATEMENT
 EXECUTE FUNCTION med_tracker.frozen_invalidate_moved_takes();

CREATE FUNCTION med_tracker.frozen_invalidate_deleted_takes() RETURNS trigger
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=pg_catalog AS $body$
DECLARE changed record; previous_source text;
BEGIN
 previous_source:=current_setting('med_tracker.frozen_proof_source',true);
 FOR changed IN SELECT DISTINCT schedule_id FROM old_takes WHERE schedule_id IS NOT NULL LOOP
  PERFORM med_tracker.frozen_read_committed();
  PERFORM set_config('med_tracker.frozen_proof_source',changed.schedule_id::text,true);
  INSERT INTO public.frozen_history_invalidations(source_schedule_id)
   SELECT changed.schedule_id WHERE EXISTS(SELECT 1 FROM public.legacy_schedule_prescriptions WHERE schedule_id=changed.schedule_id);
 END LOOP;
 PERFORM set_config('med_tracker.frozen_proof_source',coalesce(previous_source,''),true);
 RETURN NULL;
END $body$;
CREATE TRIGGER frozen_invalidate_take_delete AFTER DELETE ON public.medication_takes
 REFERENCING OLD TABLE AS old_takes FOR EACH STATEMENT
 EXECUTE FUNCTION med_tracker.frozen_invalidate_deleted_takes();

CREATE FUNCTION med_tracker.frozen_fresh_administration_guard() RETURNS trigger
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=pg_catalog AS $body$
DECLARE previous_plan text; previous_source text; previous_take text; previous_schedule text; origin_id bigint; has_parent boolean; has_fact boolean; bound_plan text;
BEGIN
 previous_plan:=current_setting('med_tracker.frozen_proof_plan',true);
 previous_source:=current_setting('med_tracker.frozen_proof_source',true);
 previous_take:=current_setting('med_tracker.sharing_take_id',true);
 previous_schedule:=current_setting('med_tracker.sharing_schedule_id',true);
 PERFORM set_config('med_tracker.frozen_proof_plan',NEW.person_schedule_id,true);
 PERFORM set_config('med_tracker.sharing_take_id',NEW.medication_take_id::text,true);
 PERFORM set_config('med_tracker.sharing_schedule_id',NEW.person_schedule_id,true);
 SELECT EXISTS(SELECT 1 FROM public.shared_schedule_adoption_facts WHERE medication_take_id=NEW.medication_take_id) INTO has_fact;
 IF has_fact THEN
  PERFORM set_config('med_tracker.sharing_take_id',coalesce(previous_take,''),true);
  PERFORM set_config('med_tracker.sharing_schedule_id',coalesce(previous_schedule,''),true);
  PERFORM set_config('med_tracker.frozen_proof_plan',coalesce(previous_plan,''),true);
  RETURN NEW;
 END IF;
 PERFORM med_tracker.frozen_read_committed();
 SELECT origin_schedule_id INTO origin_id FROM public.person_schedules WHERE id=NEW.person_schedule_id;
 IF NOT FOUND THEN RAISE EXCEPTION 'Shared administration unavailable' USING ERRCODE='42501'; END IF;
 IF origin_id IS NOT NULL THEN
  PERFORM set_config('med_tracker.frozen_proof_source',origin_id::text,true);
  SELECT adopted_person_schedule_id INTO bound_plan FROM public.legacy_schedule_prescriptions WHERE schedule_id=origin_id;
  has_parent:=FOUND;
  IF has_parent AND (bound_plan IS DISTINCT FROM NEW.person_schedule_id OR NOT med_tracker.frozen_history_proof(origin_id,true)) THEN
   PERFORM set_config('med_tracker.sharing_take_id',coalesce(previous_take,''),true);
   PERFORM set_config('med_tracker.sharing_schedule_id',coalesce(previous_schedule,''),true);
   PERFORM set_config('med_tracker.frozen_proof_source',coalesce(previous_source,''),true);
   PERFORM set_config('med_tracker.frozen_proof_plan',coalesce(previous_plan,''),true);
   RETURN NULL;
  END IF;
 END IF;
 PERFORM set_config('med_tracker.sharing_take_id',coalesce(previous_take,''),true);
 PERFORM set_config('med_tracker.sharing_schedule_id',coalesce(previous_schedule,''),true);
 PERFORM set_config('med_tracker.frozen_proof_source',coalesce(previous_source,''),true);
 PERFORM set_config('med_tracker.frozen_proof_plan',coalesce(previous_plan,''),true);
 RETURN NEW;
END $body$;
CREATE TRIGGER z_frozen_fresh_administration_guard BEFORE INSERT ON public.shared_schedule_administrations
 FOR EACH ROW EXECUTE FUNCTION med_tracker.frozen_fresh_administration_guard();

ALTER FUNCTION med_tracker.frozen_read_committed() OWNER TO med_tracker_owner;
ALTER FUNCTION med_tracker.frozen_history_proof(bigint,boolean) OWNER TO med_tracker_owner;
ALTER FUNCTION public.frozen_adoption_ready(bigint) OWNER TO med_tracker_owner;
ALTER FUNCTION med_tracker.frozen_capture_finalize() OWNER TO med_tracker_owner;
ALTER FUNCTION med_tracker.frozen_prevent_incomplete_binding() OWNER TO med_tracker_owner;
ALTER FUNCTION med_tracker.frozen_canonical_finalize() OWNER TO med_tracker_owner;
ALTER FUNCTION med_tracker.frozen_invalidate_changed_takes() OWNER TO med_tracker_owner;
ALTER FUNCTION med_tracker.frozen_invalidate_moved_takes() OWNER TO med_tracker_owner;
ALTER FUNCTION med_tracker.frozen_invalidate_deleted_takes() OWNER TO med_tracker_owner;
ALTER FUNCTION med_tracker.frozen_fresh_administration_guard() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.frozen_read_committed() FROM PUBLIC,med_tracker_app;
REVOKE ALL ON FUNCTION med_tracker.frozen_history_proof(bigint,boolean) FROM PUBLIC,med_tracker_app;
REVOKE ALL ON FUNCTION public.frozen_adoption_ready(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.frozen_adoption_ready(bigint) TO med_tracker_app;
REVOKE ALL ON FUNCTION med_tracker.frozen_capture_finalize() FROM PUBLIC,med_tracker_app;
REVOKE ALL ON FUNCTION med_tracker.frozen_prevent_incomplete_binding() FROM PUBLIC,med_tracker_app;
REVOKE ALL ON FUNCTION med_tracker.frozen_canonical_finalize() FROM PUBLIC,med_tracker_app;
REVOKE ALL ON FUNCTION med_tracker.frozen_invalidate_changed_takes() FROM PUBLIC,med_tracker_app;
REVOKE ALL ON FUNCTION med_tracker.frozen_invalidate_moved_takes() FROM PUBLIC,med_tracker_app;
REVOKE ALL ON FUNCTION med_tracker.frozen_invalidate_deleted_takes() FROM PUBLIC,med_tracker_app;
REVOKE ALL ON FUNCTION med_tracker.frozen_fresh_administration_guard() FROM PUBLIC,med_tracker_app;
