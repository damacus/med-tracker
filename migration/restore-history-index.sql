-- Restore-time live receipts: a take restored onto a still-linked canonical
-- source may mint its immutable evidence only when the receipt binds the
-- source's current plan link (source.person_schedule_id=NEW.person_schedule_id)
-- and the transaction marks that exact take through
-- med_tracker.restoring_take_id, which only the portable-history restore path
-- sets; without the marker a linked source keeps the prospective-adoption-only
-- binding, so real fresh doses cannot gain late evidence. Unlinked sources keep
-- the prospective adoption binding through plan.origin_schedule_id;
-- approval-linked (non-canonical) sources cannot mint because their plan's
-- origin_schedule_id lives in the origin household.
DROP POLICY adoption_fact_local_insert ON public.shared_schedule_adoption_facts;
CREATE POLICY adoption_fact_local_insert ON public.shared_schedule_adoption_facts FOR INSERT TO med_tracker_app
WITH CHECK (household_id=med_tracker.current_household_id() AND med_tracker.sharing_clinical_authority(person_id)
 AND EXISTS(SELECT 1 FROM public.schedules source JOIN public.person_schedules plan ON plan.id=shared_schedule_adoption_facts.person_schedule_id
  WHERE source.id=source_schedule_id AND source.household_id=shared_schedule_adoption_facts.household_id
   AND source.person_id=shared_schedule_adoption_facts.person_id
   AND (source.person_schedule_id IS NULL
    OR (source.person_schedule_id=shared_schedule_adoption_facts.person_schedule_id
     AND current_setting('med_tracker.restoring_take_id',true)=shared_schedule_adoption_facts.medication_take_id::text))
   AND source.execution_ended_at IS NULL AND plan.origin_schedule_id=source.id
   AND plan.origin_household_id=source.household_id AND plan.person_id=source.person_id));

CREATE OR REPLACE FUNCTION med_tracker.shared_administration_guard() RETURNS trigger
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=pg_catalog AS $body$
DECLARE previous_take text; previous_source text; previous_subject text; previous_schedule text;
 actual_take record; actual_source record; state public.shared_schedule_states%ROWTYPE;
 approval public.person_household_approvals%ROWTYPE; approval_epoch bigint;
 local_start timestamp; local_end timestamp; starts_at timestamp; ends_at timestamp; blocked boolean;
 effective jsonb; clinical_date date; effective_max numeric; effective_hours numeric; effective_unit text;
 original public.versions%ROWTYPE; original_fact jsonb; captured record;
BEGIN
 IF TG_TABLE_SCHEMA<>'public' OR TG_OP<>'INSERT'
  OR TG_TABLE_NAME NOT IN ('shared_schedule_administrations','shared_schedule_adoption_facts') THEN
  RAISE EXCEPTION 'Shared administration unavailable' USING ERRCODE='42501';
 END IF;
 previous_take:=current_setting('med_tracker.sharing_take_id',true);
 previous_source:=current_setting('med_tracker.sharing_source_id',true);
 previous_subject:=current_setting('med_tracker.sharing_subject_id',true);
 previous_schedule:=current_setting('med_tracker.sharing_schedule_id',true);
 PERFORM set_config('med_tracker.sharing_take_id',NEW.medication_take_id::text,true);
 IF TG_TABLE_NAME='shared_schedule_adoption_facts' THEN
  IF NEW.household_id<>med_tracker.current_household_id() THEN
   RAISE EXCEPTION 'Original administration evidence unavailable' USING ERRCODE='42501';
  END IF;
  SELECT take.* INTO actual_take FROM public.medication_takes take
  WHERE take.id=NEW.medication_take_id AND take.household_id=NEW.household_id;
  IF NOT FOUND OR actual_take.schedule_id IS NULL OR actual_take.person_medication_id IS NOT NULL THEN
   RAISE EXCEPTION 'Original administration evidence unavailable' USING ERRCODE='42501';
  END IF;
  PERFORM set_config('med_tracker.sharing_source_id',actual_take.schedule_id::text,true);
  SELECT source.* INTO actual_source FROM public.schedules source WHERE source.id=actual_take.schedule_id
   AND source.household_id=NEW.household_id AND (source.person_schedule_id IS NULL
    OR (source.person_schedule_id=NEW.person_schedule_id
     AND current_setting('med_tracker.restoring_take_id',true)=NEW.medication_take_id::text)) AND source.execution_ended_at IS NULL FOR SHARE;
  IF NOT FOUND THEN RAISE EXCEPTION 'Original administration evidence unavailable' USING ERRCODE='42501'; END IF;
  PERFORM set_config('med_tracker.sharing_subject_id',actual_source.person_id::text,true);
  IF NEW.person_schedule_id IS NULL THEN
   IF NOT med_tracker.sharing_household_actor(NEW.household_id,true)
    OR NEW.captured_by_account_id<>med_tracker.current_account_id()
    OR NEW.captured_by_membership_id<>med_tracker.current_membership_id()
    OR NOT EXISTS(SELECT 1 FROM public.accounts actor WHERE actor.id=NEW.captured_by_account_id AND actor.person_id=NEW.captured_by_person_id)
    OR NEW.guardian_declaration_id IS NOT NULL
    OR NOT EXISTS(SELECT 1 FROM public.legacy_schedule_prescriptions parent WHERE parent.schedule_id=actual_source.id
     AND parent.source_portable_id=NEW.source_portable_id AND parent.person_id=NEW.person_id AND parent.household_id=NEW.household_id
     AND parent.captured_by_account_id=NEW.captured_by_account_id AND parent.captured_by_membership_id=NEW.captured_by_membership_id
     AND parent.request_id=NEW.request_id AND parent.adopted_person_schedule_id IS NULL) THEN
    RAISE EXCEPTION 'Original administration evidence unavailable' USING ERRCODE='42501';
   END IF;
  ELSIF NOT med_tracker.sharing_household_actor(NEW.household_id,false)
   OR NOT med_tracker.sharing_clinical_authority(actual_source.person_id)
   OR NEW.captured_by_account_id<>med_tracker.current_account_id()
   OR NEW.captured_by_membership_id<>med_tracker.current_membership_id()
   OR NOT EXISTS(SELECT 1 FROM public.accounts actor WHERE actor.id=NEW.captured_by_account_id AND actor.person_id=NEW.captured_by_person_id)
   OR (NEW.captured_by_person_id=actual_source.person_id AND NEW.guardian_declaration_id IS NOT NULL)
   OR (NEW.captured_by_person_id<>actual_source.person_id AND NEW.guardian_declaration_id IS DISTINCT FROM med_tracker.sharing_guardian(actual_source.person_id))
   OR NOT EXISTS(SELECT 1 FROM public.person_schedules plan WHERE plan.id=NEW.person_schedule_id
    AND plan.origin_schedule_id=actual_source.id AND plan.origin_household_id=actual_source.household_id AND plan.person_id=actual_source.person_id) THEN
   RAISE EXCEPTION 'Original administration evidence unavailable' USING ERRCODE='42501';
  END IF;
  IF (NEW.source_schedule_id,NEW.source_portable_id,NEW.person_id,NEW.household_id)
   IS DISTINCT FROM (actual_source.id,actual_source.portable_id,actual_source.person_id,actual_source.household_id)
   OR (NEW.medication_take_id,NEW.take_portable_id,NEW.client_uuid,NEW.taken_at,NEW.dose_amount,NEW.dose_unit,
    NEW.taken_from_medication_id,NEW.taken_from_location_id,NEW.take_created_at,NEW.take_updated_at)
    IS DISTINCT FROM (actual_take.id,actual_take.portable_id,actual_take.client_uuid,actual_take.taken_at,actual_take.dose_amount,actual_take.dose_unit,
    actual_take.taken_from_medication_id,actual_take.taken_from_location_id,actual_take.created_at,actual_take.updated_at)
   OR NEW.dose_amount<=0 OR NEW.fact_hash IS DISTINCT FROM med_tracker.adoption_fact_hash(NEW::public.shared_schedule_adoption_facts) THEN
   RAISE EXCEPTION 'Original administration evidence unavailable' USING ERRCODE='42501';
  END IF;
  SELECT * INTO original FROM public.versions WHERE item_type='MedicationTake' AND item_id=actual_take.id AND event='create' ORDER BY id LIMIT 1;
  IF NOT FOUND OR original.id<>NEW.original_version_id OR original.household_id IS DISTINCT FROM NEW.household_id
   OR original.whodunnit IS DISTINCT FROM original.audit_context->>'actor_account_id'
   OR original.actor_membership_id::text IS DISTINCT FROM original.audit_context->>'actor_membership_id'
   OR original.household_id::text IS DISTINCT FROM original.audit_context->>'household_id'
   OR original.request_id IS DISTINCT FROM original.audit_context->>'request_id' THEN
   RAISE EXCEPTION 'Original administration evidence unavailable' USING ERRCODE='42501';
  END IF;
  IF original.object IS NOT NULL THEN original_fact:=original.object::jsonb;
  ELSE SELECT jsonb_object_agg(change.key,change.value->1) INTO original_fact FROM jsonb_each(original.object_changes::jsonb) change
   WHERE jsonb_typeof(change.value)='array' AND jsonb_array_length(change.value)=2 AND change.value->0='null'::jsonb; END IF;
  IF (original_fact->>'id')::bigint IS DISTINCT FROM actual_take.id
   OR original_fact->>'portable_id' IS DISTINCT FROM actual_take.portable_id
   OR original_fact->>'client_uuid' IS DISTINCT FROM actual_take.client_uuid
   OR (original_fact->>'household_id')::bigint IS DISTINCT FROM actual_take.household_id
   OR (original_fact->>'schedule_id')::bigint IS DISTINCT FROM actual_take.schedule_id
   OR (original_fact->>'person_medication_id')::bigint IS DISTINCT FROM actual_take.person_medication_id
   OR (original_fact->>'taken_from_medication_id')::bigint IS DISTINCT FROM actual_take.taken_from_medication_id
   OR (original_fact->>'taken_from_location_id')::bigint IS DISTINCT FROM actual_take.taken_from_location_id
   OR (original_fact->>'dose_amount')::numeric IS DISTINCT FROM actual_take.dose_amount
   OR original_fact->>'dose_unit' IS DISTINCT FROM actual_take.dose_unit
   OR NOT med_tracker.adoption_audit_time_matches(original_fact->>'taken_at',actual_take.taken_at)
   OR NOT med_tracker.adoption_audit_time_matches(original_fact->>'created_at',actual_take.created_at)
   OR NOT med_tracker.adoption_audit_time_matches(original_fact->>'updated_at',actual_take.updated_at) THEN
   RAISE EXCEPTION 'Original administration evidence unavailable' USING ERRCODE='42501';
  END IF;
  PERFORM set_config('med_tracker.sharing_schedule_id',COALESCE(previous_schedule,''),true);
  PERFORM set_config('med_tracker.sharing_subject_id',COALESCE(previous_subject,''),true);
  PERFORM set_config('med_tracker.sharing_source_id',COALESCE(previous_source,''),true);
  PERFORM set_config('med_tracker.sharing_take_id',COALESCE(previous_take,''),true);
  RETURN NEW;
 END IF;
 PERFORM set_config('med_tracker.sharing_schedule_id',NEW.person_schedule_id,true);
 SELECT take.* INTO actual_take
 FROM public.medication_takes take WHERE take.id=NEW.medication_take_id AND take.household_id=NEW.household_id;
 IF NOT FOUND OR actual_take.schedule_id IS NULL OR actual_take.taken_at IS NULL
  OR actual_take.taken_at IS DISTINCT FROM NEW.taken_at OR actual_take.dose_amount IS NULL OR actual_take.dose_amount<=0 THEN
  RAISE EXCEPTION 'Shared administration unavailable' USING ERRCODE='42501';
 END IF;
 PERFORM set_config('med_tracker.sharing_source_id',actual_take.schedule_id::text,true);
 SELECT fact.* INTO captured FROM public.shared_schedule_adoption_facts fact WHERE fact.medication_take_id=NEW.medication_take_id;
 IF FOUND AND captured.person_schedule_id IS NULL THEN
  IF NEW.occurrence_starts_on IS NOT NULL OR NEW.occurrence_position IS NOT NULL
   OR NEW.household_id<>captured.household_id
   OR captured.source_schedule_id<>actual_take.schedule_id
   OR captured.fact_hash IS DISTINCT FROM (SELECT med_tracker.adoption_fact_hash(fact)
    FROM public.shared_schedule_adoption_facts fact WHERE fact.medication_take_id=NEW.medication_take_id)
   OR (captured.medication_take_id,captured.take_portable_id,captured.client_uuid,captured.taken_at,captured.dose_amount,captured.dose_unit,
    captured.taken_from_medication_id,captured.taken_from_location_id,captured.take_created_at,captured.take_updated_at)
    IS DISTINCT FROM (actual_take.id,actual_take.portable_id,actual_take.client_uuid,actual_take.taken_at,actual_take.dose_amount,actual_take.dose_unit,
    actual_take.taken_from_medication_id,actual_take.taken_from_location_id,actual_take.created_at,actual_take.updated_at)
   OR NOT EXISTS(SELECT 1 FROM public.legacy_schedule_prescriptions parent
    WHERE parent.schedule_id=captured.source_schedule_id AND parent.adopted_person_schedule_id=NEW.person_schedule_id
     AND parent.person_id=captured.person_id AND parent.household_id=captured.household_id) THEN
   RAISE EXCEPTION 'Original administration evidence unavailable' USING ERRCODE='42501';
  END IF;
  PERFORM set_config('med_tracker.sharing_subject_id',captured.person_id::text,true);
  SELECT * INTO state FROM public.shared_schedule_states WHERE id=NEW.person_schedule_id FOR UPDATE;
  IF NOT FOUND OR state.person_id<>captured.person_id OR NOT med_tracker.sharing_clinical_authority(state.person_id) THEN
   RAISE EXCEPTION 'Original administration evidence unavailable' USING ERRCODE='42501';
  END IF;
  PERFORM set_config('med_tracker.sharing_schedule_id',COALESCE(previous_schedule,''),true);
  PERFORM set_config('med_tracker.sharing_subject_id',COALESCE(previous_subject,''),true);
  PERFORM set_config('med_tracker.sharing_source_id',COALESCE(previous_source,''),true);
  PERFORM set_config('med_tracker.sharing_take_id',COALESCE(previous_take,''),true);
  RETURN NEW;
 END IF;
 IF NEW.household_id<>med_tracker.current_household_id()
  OR NOT med_tracker.sharing_household_actor(NEW.household_id,false)
  OR NEW.occurrence_starts_on IS NOT NULL OR NEW.occurrence_position IS NOT NULL THEN
  RAISE EXCEPTION 'Shared administration unavailable' USING ERRCODE='42501';
 END IF;
 IF FOUND THEN
  SELECT source.* INTO actual_source FROM public.schedules source WHERE source.id=actual_take.schedule_id
   AND source.household_id=NEW.household_id AND source.execution_ended_at IS NULL FOR SHARE;
  IF NOT FOUND OR actual_source.person_schedule_id IS DISTINCT FROM NEW.person_schedule_id OR NOT actual_source.canonical_origin
   OR captured.person_schedule_id IS DISTINCT FROM NEW.person_schedule_id
   OR captured.source_schedule_id<>actual_source.id OR captured.source_portable_id<>actual_source.portable_id
   OR captured.person_id<>actual_source.person_id OR captured.household_id<>NEW.household_id
   OR captured.captured_by_account_id<>med_tracker.current_account_id()
   OR captured.captured_by_membership_id<>med_tracker.current_membership_id()
   OR captured.fact_hash IS DISTINCT FROM (SELECT med_tracker.adoption_fact_hash(fact)
    FROM public.shared_schedule_adoption_facts fact WHERE fact.medication_take_id=NEW.medication_take_id)
   OR (captured.medication_take_id,captured.take_portable_id,captured.client_uuid,captured.taken_at,captured.dose_amount,captured.dose_unit,
    captured.taken_from_medication_id,captured.taken_from_location_id,captured.take_created_at,captured.take_updated_at)
    IS DISTINCT FROM (actual_take.id,actual_take.portable_id,actual_take.client_uuid,actual_take.taken_at,actual_take.dose_amount,actual_take.dose_unit,
    actual_take.taken_from_medication_id,actual_take.taken_from_location_id,actual_take.created_at,actual_take.updated_at)
   OR EXISTS(SELECT 1 FROM public.shared_schedule_administrations prior WHERE prior.medication_take_id=NEW.medication_take_id) THEN
   RAISE EXCEPTION 'Original administration evidence unavailable' USING ERRCODE='42501';
  END IF;
  PERFORM set_config('med_tracker.sharing_subject_id',actual_source.person_id::text,true);
  SELECT * INTO state FROM public.shared_schedule_states WHERE id=NEW.person_schedule_id FOR UPDATE;
  IF NOT FOUND OR state.person_id<>actual_source.person_id OR NOT med_tracker.sharing_clinical_authority(state.person_id)
   OR (captured.captured_by_person_id=state.person_id AND captured.guardian_declaration_id IS NOT NULL)
   OR (captured.captured_by_person_id<>state.person_id AND captured.guardian_declaration_id IS DISTINCT FROM med_tracker.sharing_guardian(state.person_id)) THEN
   RAISE EXCEPTION 'Original administration evidence unavailable' USING ERRCODE='42501';
  END IF;
  SELECT link.approval_id INTO approval_epoch FROM public.shared_schedule_executions link
   WHERE link.schedule_id=actual_source.id AND link.person_schedule_id=state.id AND link.household_id=NEW.household_id AND link.person_id=state.person_id;
  IF NOT FOUND THEN RAISE EXCEPTION 'Original administration evidence unavailable' USING ERRCODE='42501'; END IF;
  IF approval_epoch IS NOT NULL THEN
   SELECT * INTO approval FROM public.person_household_approvals WHERE id=approval_epoch
    AND person_id=state.person_id AND household_id=NEW.household_id FOR SHARE;
   IF NOT FOUND OR approval.ended_at IS NOT NULL OR NOT med_tracker.sharing_current_member(state.person_id) THEN
    RAISE EXCEPTION 'Original administration evidence unavailable' USING ERRCODE='42501';
   END IF;
  ELSIF EXISTS(SELECT 1 FROM public.person_household_approvals prior WHERE prior.person_id=state.person_id AND prior.household_id=NEW.household_id) THEN
   RAISE EXCEPTION 'Original administration evidence unavailable' USING ERRCODE='42501';
  END IF;
  PERFORM set_config('med_tracker.sharing_schedule_id',COALESCE(previous_schedule,''),true);
  PERFORM set_config('med_tracker.sharing_subject_id',COALESCE(previous_subject,''),true);
  PERFORM set_config('med_tracker.sharing_source_id',COALESCE(previous_source,''),true);
  PERFORM set_config('med_tracker.sharing_take_id',COALESCE(previous_take,''),true);
  RETURN NEW;
 END IF;
 SELECT source.id,source.person_id,source.person_schedule_id,source.sharing_approval_id,source.clinical_revision,source.dose_unit,source.canonical_origin INTO actual_source
 FROM public.schedules source WHERE source.id=actual_take.schedule_id AND source.household_id=NEW.household_id
  AND source.execution_ended_at IS NULL AND source.active AND source.retired_at IS NULL;
 IF NOT FOUND OR actual_source.person_schedule_id IS DISTINCT FROM NEW.person_schedule_id THEN
  RAISE EXCEPTION 'Shared administration unavailable' USING ERRCODE='42501';
 END IF;
 PERFORM set_config('med_tracker.sharing_subject_id',actual_source.person_id::text,true);
 SELECT * INTO state FROM public.shared_schedule_states WHERE id=NEW.person_schedule_id FOR UPDATE;
 IF NOT FOUND OR state.person_id<>actual_source.person_id OR NOT state.active OR state.retired_at IS NOT NULL
  OR actual_source.clinical_revision IS DISTINCT FROM state.clinical_revision THEN
  RAISE EXCEPTION 'Shared administration unavailable' USING ERRCODE='42501';
 END IF;
 clinical_date:=timezone(state.origin_time_zone,actual_take.taken_at AT TIME ZONE 'UTC')::date;
 effective:=state.schedule_config;
 IF state.schedule_type=5 THEN
  SELECT step.value INTO effective FROM jsonb_array_elements(state.schedule_config->'taper_steps') WITH ORDINALITY step(value,position)
  WHERE clinical_date BETWEEN (step.value->>'start_date')::date AND (step.value->>'end_date')::date ORDER BY step.position LIMIT 1;
  IF NOT FOUND THEN RAISE EXCEPTION 'Shared administration unavailable' USING ERRCODE='42501'; END IF;
 END IF;
 effective_unit:=COALESCE(effective->>'unit',effective->>'dose_unit',state.schedule_config->>'unit',state.schedule_config->>'dose_unit',state.dose_unit);
 effective_max:=COALESCE(trunc(COALESCE(effective->>'max_daily_doses',effective->>'max_doses',effective->>'max')::numeric),
  trunc(COALESCE(state.schedule_config->>'max_daily_doses',state.schedule_config->>'max_doses',state.schedule_config->>'max')::numeric),state.max_daily_doses);
 effective_hours:=COALESCE(COALESCE(effective->>'min_hours_between_doses',effective->>'min_hours',effective->>'minimum_hours')::numeric,
  COALESCE(state.schedule_config->>'min_hours_between_doses',state.schedule_config->>'min_hours',state.schedule_config->>'minimum_hours')::numeric,state.min_hours_between_doses);
 IF state.schedule_type=0 AND (SELECT count(*) FROM jsonb_array_elements(CASE WHEN jsonb_typeof(effective->'times')='array' THEN effective->'times' ELSE '[]'::jsonb END) item(value)
  WHERE item.value<>'null'::jsonb AND item.value<>'""'::jsonb)=1 THEN effective_hours:=NULL; END IF;
 IF actual_take.dose_unit IS DISTINCT FROM effective_unit OR clinical_date<state.start_date OR clinical_date>state.end_date THEN
  RAISE EXCEPTION 'Shared administration unavailable' USING ERRCODE='42501';
 END IF;
 SELECT COALESCE(actual_source.sharing_approval_id,link.approval_id) INTO approval_epoch
 FROM (SELECT 1) seed LEFT JOIN public.shared_schedule_executions link ON link.schedule_id=actual_source.id;
 IF approval_epoch IS NOT NULL THEN
  SELECT * INTO approval FROM public.person_household_approvals
  WHERE id=approval_epoch AND household_id=NEW.household_id AND person_id=state.person_id FOR SHARE;
  IF NOT FOUND OR approval.ended_at IS NOT NULL OR actual_take.taken_at<approval.approved_at
   OR NOT med_tracker.sharing_current_member(state.person_id) THEN
   RAISE EXCEPTION 'Shared administration unavailable' USING ERRCODE='42501';
  END IF;
 ELSE
  IF NOT actual_source.canonical_origin
   OR NOT EXISTS(SELECT 1 FROM public.shared_schedule_executions link WHERE link.schedule_id=actual_source.id
    AND link.person_schedule_id=state.id AND link.person_id=state.person_id AND link.household_id=NEW.household_id AND link.approval_id IS NULL)
   OR EXISTS(SELECT 1 FROM public.person_household_approvals prior WHERE prior.person_id=state.person_id AND prior.household_id=NEW.household_id) THEN
   RAISE EXCEPTION 'Shared administration unavailable' USING ERRCODE='42501';
  END IF;
 END IF;
 IF NOT med_tracker.sharing_household_actor(NEW.household_id,false)
  OR NOT EXISTS(SELECT 1 FROM public.schedules source WHERE source.id=actual_source.id
   AND source.household_id=NEW.household_id AND source.person_schedule_id=state.id
   AND source.execution_ended_at IS NULL AND source.active AND source.retired_at IS NULL
   AND source.clinical_revision=state.clinical_revision) THEN
  RAISE EXCEPTION 'Shared administration unavailable' USING ERRCODE='42501';
 END IF;
 local_start:=date_trunc(CASE state.dose_cycle WHEN 1 THEN 'week' WHEN 2 THEN 'month' ELSE 'day' END,
  timezone(state.origin_time_zone,actual_take.taken_at AT TIME ZONE 'UTC'));
 local_end:=local_start+CASE state.dose_cycle WHEN 1 THEN interval '1 week' WHEN 2 THEN interval '1 month' ELSE interval '1 day' END;
 starts_at:=timezone('UTC',local_start AT TIME ZONE state.origin_time_zone);
 ends_at:=timezone('UTC',local_end AT TIME ZONE state.origin_time_zone);
 PERFORM set_config('med_tracker.sharing_schedule_id',state.id,true);
 SELECT (effective_max IS NOT NULL AND (SELECT count(*) FROM public.shared_schedule_administrations
   WHERE person_schedule_id=state.id AND taken_at>=starts_at AND taken_at<ends_at)>=greatest(effective_max,0))
  OR (effective_hours IS NOT NULL AND effective_hours>0 AND EXISTS(SELECT 1 FROM public.shared_schedule_administrations
   WHERE person_schedule_id=state.id AND abs(extract(epoch FROM actual_take.taken_at-taken_at))<effective_hours*3600)) INTO blocked;
 PERFORM set_config('med_tracker.sharing_schedule_id',COALESCE(previous_schedule,''),true);
 PERFORM set_config('med_tracker.sharing_subject_id',COALESCE(previous_subject,''),true);
 PERFORM set_config('med_tracker.sharing_source_id',COALESCE(previous_source,''),true);
 PERFORM set_config('med_tracker.sharing_take_id',COALESCE(previous_take,''),true);
 IF COALESCE(blocked,false) THEN RETURN NULL; END IF;
 RETURN NEW;
END $body$;
ALTER FUNCTION med_tracker.shared_administration_guard() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.shared_administration_guard() FROM PUBLIC,med_tracker_app;
