CREATE FUNCTION med_tracker.guard_care_source_identity() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER SET search_path = pg_catalog
AS $$
BEGIN
  IF NEW.person_id IS DISTINCT FROM OLD.person_id
    OR NEW.household_id IS DISTINCT FROM OLD.household_id THEN
    RAISE EXCEPTION 'Care source identity is immutable' USING ERRCODE = '23514';
  END IF;
  RETURN NEW;
END
$$;

REVOKE ALL ON FUNCTION med_tracker.guard_care_source_identity() FROM PUBLIC, med_tracker_app;

CREATE TRIGGER schedule_identity_guard
BEFORE UPDATE OF person_id, household_id ON public.schedules
FOR EACH ROW EXECUTE FUNCTION med_tracker.guard_care_source_identity();

CREATE TRIGGER assignment_identity_guard
BEFORE UPDATE OF person_id, household_id ON public.person_medications
FOR EACH ROW EXECUTE FUNCTION med_tracker.guard_care_source_identity();

CREATE FUNCTION med_tracker.guard_medication_take_identity() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER SET search_path = pg_catalog
AS $$
BEGIN
  IF ROW(NEW.id, NEW.household_id, NEW.schedule_id, NEW.person_medication_id,
         NEW.taken_at, NEW.dose_amount, NEW.dose_unit, NEW.taken_from_medication_id,
         NEW.taken_from_location_id, NEW.client_uuid, NEW.portable_id, NEW.created_at)
    IS DISTINCT FROM
     ROW(OLD.id, OLD.household_id, OLD.schedule_id, OLD.person_medication_id,
         OLD.taken_at, OLD.dose_amount, OLD.dose_unit, OLD.taken_from_medication_id,
         OLD.taken_from_location_id, OLD.client_uuid, OLD.portable_id, OLD.created_at) THEN
    RAISE EXCEPTION 'Recorded medication administration is immutable' USING ERRCODE = '23514';
  END IF;
  RETURN NEW;
END
$$;

REVOKE ALL ON FUNCTION med_tracker.guard_medication_take_identity() FROM PUBLIC, med_tracker_app;

CREATE TRIGGER medication_take_identity_guard
BEFORE UPDATE ON public.medication_takes
FOR EACH ROW EXECUTE FUNCTION med_tracker.guard_medication_take_identity();

ALTER TABLE public.clinical_history_entries
  ADD CONSTRAINT clinical_history_revision_positive CHECK (event_revision > 0),
  ADD CONSTRAINT clinical_history_snapshot_version CHECK (snapshot_version = 1),
  ADD CONSTRAINT clinical_history_snapshot_object CHECK (jsonb_typeof(snapshot) = 'object');
ALTER TABLE public.clinical_history_entries ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.clinical_history_entries FORCE ROW LEVEL SECURITY;
CREATE POLICY history_owner ON public.clinical_history_entries TO med_tracker_owner USING (true) WITH CHECK (true);
REVOKE ALL ON public.clinical_history_entries FROM PUBLIC, med_tracker_app, med_tracker_audit_exporter, med_tracker_audit_verifier;

CREATE FUNCTION med_tracker.guard_take_history() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER SET search_path = pg_catalog
AS $$ BEGIN
  RAISE EXCEPTION 'Original clinical history is immutable' USING ERRCODE='23514';
END $$;
REVOKE ALL ON FUNCTION med_tracker.guard_take_history() FROM PUBLIC, med_tracker_app;
CREATE TRIGGER clinical_history_immutable BEFORE UPDATE OR DELETE ON public.clinical_history_entries
FOR EACH ROW EXECUTE FUNCTION med_tracker.guard_take_history();

CREATE FUNCTION med_tracker.append_take_history(target_take bigint, expected_medication bigint, credential_method text DEFAULT NULL, credential_reference text DEFAULT NULL) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
DECLARE taken public.medication_takes%ROWTYPE; subject bigint; prescribed bigint;
  source_portable text; source_kind text; administration text; actor public.household_memberships%ROWTYPE;
  medicine public.medications%ROWTYPE; stock public.medications%ROWTYPE; storage public.locations%ROWTYPE;
  subject_portable text; subject_label text; actor_label text; document jsonb;
BEGIN
  PERFORM med_tracker.lock_current_household();
  SELECT * INTO actor FROM public.household_memberships m
    WHERE m.id=med_tracker.current_membership_id() AND m.person_id=med_tracker.current_person_id()
      AND m.household_id=med_tracker.current_household_id() AND m.status='active' AND m.revoked_at IS NULL;
  IF NOT FOUND THEN RAISE EXCEPTION 'Current clinical actor required' USING ERRCODE='42501'; END IF;
  SELECT * INTO taken FROM public.medication_takes t WHERE t.id=target_take AND t.household_id=actor.household_id FOR UPDATE;
  IF NOT FOUND THEN RAISE EXCEPTION 'Local take unavailable' USING ERRCODE='42501'; END IF;
  IF EXISTS(SELECT 1 FROM public.clinical_history_entries h WHERE h.take_id=taken.id AND h.event_revision=1) THEN RETURN; END IF;
  IF taken.schedule_id IS NOT NULL THEN
    SELECT s.person_id,s.medication_id,s.portable_id,CASE WHEN s.schedule_type=4 THEN 'as_needed' ELSE 'scheduled' END
      INTO subject,prescribed,source_portable,administration FROM public.schedules s
      WHERE s.id=taken.schedule_id AND s.household_id=taken.household_id FOR SHARE;
    source_kind := 'schedule';
  ELSE
    SELECT s.person_id,s.medication_id,s.portable_id,CASE WHEN s.administration_kind=0 THEN 'routine' ELSE 'as_needed' END
      INTO subject,prescribed,source_portable,administration FROM public.person_medications s
      WHERE s.id=taken.person_medication_id AND s.household_id=taken.household_id FOR SHARE;
    source_kind := 'person_medication';
  END IF;
  IF subject IS NULL OR prescribed IS DISTINCT FROM expected_medication OR taken.taken_at IS NULL
    OR NOT EXISTS(SELECT 1 FROM public.household_memberships m WHERE m.person_id=subject AND m.household_id=taken.household_id AND m.status='active' AND m.revoked_at IS NULL) THEN
    RAISE EXCEPTION 'Clinical source changed or unavailable' USING ERRCODE='23514';
  END IF;
  IF (credential_method IS NULL) <> (credential_reference IS NULL)
    OR (credential_method IS NOT NULL AND credential_method NOT IN ('personal_api_key','api_session','api_app_token','oauth','browser_session'))
    OR length(credential_reference)>512 THEN
    RAISE EXCEPTION 'Invalid clinical credential attribution' USING ERRCODE='23514';
  END IF;
  SELECT * INTO medicine FROM public.medications m WHERE m.id=prescribed AND m.household_id=taken.household_id FOR SHARE;
  SELECT * INTO stock FROM public.medications m WHERE m.id=taken.taken_from_medication_id AND m.household_id=taken.household_id FOR SHARE;
  SELECT * INTO storage FROM public.locations l WHERE l.id=taken.taken_from_location_id AND l.household_id=taken.household_id;
  SELECT p.portable_id,med_tracker.person_display_name(p.id) INTO subject_portable,subject_label FROM public.people p WHERE p.id=subject;
  actor_label := med_tracker.person_display_name(actor.person_id);
  IF medicine.id IS NULL OR subject_label IS NULL OR actor_label IS NULL THEN
    RAISE EXCEPTION 'Clinical attribution unavailable' USING ERRCODE='23514';
  END IF;
  document := jsonb_build_object(
    'event',jsonb_build_object('portable_id',taken.portable_id,'client_uuid',taken.client_uuid,
      'created_at',taken.created_at,'updated_at',taken.updated_at,'taken_at',taken.taken_at,
      'dose_amount',taken.dose_amount,'dose_unit',taken.dose_unit,'schedule_id',taken.schedule_id,
      'person_medication_id',taken.person_medication_id,'taken_from_medication_id',taken.taken_from_medication_id,
      'taken_from_location_id',taken.taken_from_location_id),
    'subject',jsonb_build_object('portable_id',subject_portable,'name',subject_label),
    'actor',jsonb_build_object('kind','known','person_id',actor.person_id,'membership_id',actor.id,'account_id',med_tracker.current_account_id(),'name',actor_label),
    'source',jsonb_build_object('kind',source_kind,'portable_id',source_portable,'administration',administration),
    'medicine',jsonb_build_object('id',medicine.id,'portable_id',medicine.portable_id,'name',COALESCE(NULLIF(medicine.friendly_name,''),medicine.name)),
    'stock',jsonb_build_object('portable_id',stock.portable_id,'name',COALESCE(NULLIF(stock.friendly_name,''),stock.name)),
    'location',jsonb_build_object('portable_id',storage.portable_id,'name',storage.name),
    'credential',CASE WHEN credential_method IS NULL THEN NULL ELSE jsonb_build_object('method',credential_method,'reference',credential_reference) END);
  INSERT INTO public.clinical_history_entries(take_id,person_id,origin_household_id,occurred_at,snapshot)
    VALUES(taken.id,subject,taken.household_id,taken.taken_at AT TIME ZONE 'UTC',document);
END $$;
REVOKE ALL ON FUNCTION med_tracker.append_take_history(bigint,bigint,text,text) FROM PUBLIC, med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.append_take_history(bigint,bigint,text,text) TO med_tracker_app;

CREATE FUNCTION med_tracker.require_original_take_history() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$ BEGIN
  IF NOT EXISTS(SELECT 1 FROM public.clinical_history_entries h
    WHERE h.take_id=NEW.id AND h.origin_household_id=NEW.household_id AND h.event_revision=1) THEN
    RAISE EXCEPTION 'Original clinical history required' USING ERRCODE='23514';
  END IF;
  RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION med_tracker.require_original_take_history() FROM PUBLIC, med_tracker_app;
CREATE CONSTRAINT TRIGGER take_history_required AFTER INSERT ON public.medication_takes
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION med_tracker.require_original_take_history();

CREATE VIEW med_tracker.local_take_history_projection WITH (security_barrier=true) AS
SELECT t.id,t.portable_id,t.client_uuid,t.created_at,t.updated_at,t.taken_at,t.dose_amount,t.dose_unit,t.household_id,
  h.person_id,h.snapshot->'subject'->>'portable_id' AS person_portable_id,h.snapshot->'subject'->>'name' AS subject_name,
  t.schedule_id,CASE WHEN t.schedule_id IS NOT NULL THEN h.snapshot->'source'->>'portable_id' END AS schedule_portable_id,
  t.person_medication_id,CASE WHEN t.person_medication_id IS NOT NULL THEN h.snapshot->'source'->>'portable_id' END AS person_medication_portable_id,
  (h.snapshot->'medicine'->>'id')::bigint AS medication_id,h.snapshot->'medicine'->>'portable_id' AS medication_portable_id,
  h.snapshot->'medicine'->>'name' AS medication_name,h.snapshot->'source'->>'administration' AS administration,
  t.taken_from_medication_id,h.snapshot->'stock'->>'portable_id' AS taken_from_medication_portable_id,
  t.taken_from_location_id,h.snapshot->'location'->>'portable_id' AS taken_from_location_portable_id,
  h.snapshot->'location'->>'name' AS location_name,
  h.snapshot->'actor'->>'kind' AS actor_kind,(h.snapshot->'actor'->>'person_id')::bigint AS actor_person_id,
  (h.snapshot->'actor'->>'membership_id')::bigint AS actor_membership_id,h.snapshot->'actor'->>'name' AS actor_name
FROM public.clinical_history_entries h JOIN public.medication_takes t ON t.id=h.take_id AND t.household_id=h.origin_household_id
WHERE h.event_revision=1 AND h.origin_household_id=med_tracker.current_household_id()
  AND med_tracker.has_household_access(h.origin_household_id);
REVOKE ALL ON med_tracker.local_take_history_projection FROM PUBLIC, med_tracker_app;

CREATE FUNCTION med_tracker.local_take_history(requested_ids bigint[]) RETURNS SETOF med_tracker.local_take_history_projection
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$ BEGIN
  IF requested_ids IS NULL OR cardinality(requested_ids)>100 THEN
    RAISE EXCEPTION 'History page is invalid' USING ERRCODE='23514';
  END IF;
  RETURN QUERY SELECT h.* FROM med_tracker.local_take_history_projection h WHERE h.id=ANY(requested_ids) ORDER BY h.id;
END $$;
REVOKE ALL ON FUNCTION med_tracker.local_take_history(bigint[]) FROM PUBLIC, med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.local_take_history(bigint[]) TO med_tracker_app;

CREATE FUNCTION med_tracker.local_dose_timing(target_household bigint, subjects bigint[], medicines bigint[], starts_at timestamp[], ends_at timestamp[], candidate timestamp)
RETURNS TABLE(window_index bigint, cycle_count bigint, latest_at timestamp)
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$ BEGIN
  IF target_household IS NULL OR target_household IS DISTINCT FROM med_tracker.current_household_id()
    OR NOT med_tracker.has_household_access(target_household)
    OR NOT EXISTS(SELECT 1 FROM public.household_memberships m WHERE m.id=med_tracker.current_membership_id()
      AND m.person_id=med_tracker.current_person_id() AND m.household_id=target_household AND m.status='active' AND m.revoked_at IS NULL) THEN
    RAISE EXCEPTION 'Current clinical scope required' USING ERRCODE='42501';
  END IF;
  IF subjects IS NULL OR medicines IS NULL OR starts_at IS NULL OR ends_at IS NULL OR candidate IS NULL
    OR cardinality(subjects)<>cardinality(medicines) OR cardinality(subjects)<>cardinality(starts_at) OR cardinality(subjects)<>cardinality(ends_at)
    OR EXISTS(SELECT 1 FROM unnest(subjects,medicines,starts_at,ends_at) w(subject,medicine,starts,ends)
      WHERE w.subject IS NULL OR w.medicine IS NULL OR w.starts IS NULL OR w.ends IS NULL OR w.starts>=w.ends) THEN
    RAISE EXCEPTION 'Timing window is invalid' USING ERRCODE='23514';
  END IF;
  IF EXISTS(SELECT 1 FROM unnest(subjects,medicines) w(subject,medicine)
    WHERE NOT EXISTS(SELECT 1 FROM public.household_memberships m WHERE m.person_id=w.subject AND m.household_id=target_household AND m.status='active' AND m.revoked_at IS NULL)
       OR NOT EXISTS(SELECT 1 FROM public.medications m WHERE m.id=w.medicine AND m.household_id=target_household)) THEN
    RAISE EXCEPTION 'Local clinical scope required' USING ERRCODE='42501';
  END IF;
  RETURN QUERY SELECT w.ordinality,
    (SELECT count(*) FROM public.clinical_history_entries h JOIN public.medication_takes t ON t.id=h.take_id AND t.household_id=h.origin_household_id
      WHERE h.event_revision=1 AND h.origin_household_id=target_household AND h.person_id=w.subject
        AND (h.snapshot->'medicine'->>'id')::bigint=w.medicine AND t.taken_at>=w.starts AND t.taken_at<w.ends),
    (SELECT max(t.taken_at) FROM public.clinical_history_entries h JOIN public.medication_takes t ON t.id=h.take_id AND t.household_id=h.origin_household_id
      WHERE h.event_revision=1 AND h.origin_household_id=target_household AND h.person_id=w.subject
        AND (h.snapshot->'medicine'->>'id')::bigint=w.medicine AND t.taken_at<=candidate)
    FROM unnest(subjects,medicines,starts_at,ends_at) WITH ORDINALITY w(subject,medicine,starts,ends,ordinality);
END $$;
REVOKE ALL ON FUNCTION med_tracker.local_dose_timing(bigint,bigint[],bigint[],timestamp[],timestamp[],timestamp) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.local_dose_timing(bigint,bigint[],bigint[],timestamp[],timestamp[],timestamp) TO med_tracker_app;

CREATE FUNCTION med_tracker.local_dashboard_take_history(target_household bigint, subjects bigint[], starts_at timestamp, ends_at timestamp)
RETURNS TABLE(id bigint,person_id bigint,subject_name text,medication_id bigint,medication_name text,schedule_id bigint,person_medication_id bigint,administration text,dose_amount numeric,dose_unit text,taken_at timestamp)
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$ BEGIN
  IF target_household IS NULL OR target_household IS DISTINCT FROM med_tracker.current_household_id()
    OR NOT med_tracker.has_household_access(target_household)
    OR NOT EXISTS(SELECT 1 FROM public.household_memberships m WHERE m.id=med_tracker.current_membership_id()
      AND m.person_id=med_tracker.current_person_id() AND m.household_id=target_household AND m.status='active' AND m.revoked_at IS NULL) THEN
    RAISE EXCEPTION 'Current clinical scope required' USING ERRCODE='42501';
  END IF;
  IF subjects IS NULL OR starts_at IS NULL OR ends_at IS NULL OR starts_at>=ends_at OR array_position(subjects,NULL) IS NOT NULL THEN
    RAISE EXCEPTION 'Dashboard window is invalid' USING ERRCODE='23514';
  END IF;
  IF EXISTS(SELECT 1 FROM unnest(subjects) requested(person_id) WHERE NOT EXISTS(
    SELECT 1 FROM public.household_memberships m WHERE m.person_id=requested.person_id AND m.household_id=target_household AND m.status='active' AND m.revoked_at IS NULL)) THEN
    RAISE EXCEPTION 'Local clinical scope required' USING ERRCODE='42501';
  END IF;
  RETURN QUERY SELECT h.id,h.person_id,h.subject_name,h.medication_id,h.medication_name,h.schedule_id,h.person_medication_id,h.administration,h.dose_amount,h.dose_unit::text,h.taken_at
    FROM med_tracker.local_take_history_projection h
    WHERE h.person_id=ANY(subjects) AND h.taken_at>=starts_at AND h.taken_at<ends_at
    ORDER BY h.taken_at,h.id;
END $$;
REVOKE ALL ON FUNCTION med_tracker.local_dashboard_take_history(bigint,bigint[],timestamp,timestamp) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.local_dashboard_take_history(bigint,bigint[],timestamp,timestamp) TO med_tracker_app;

CREATE FUNCTION med_tracker.local_take_report(subject bigint, starts_at timestamp, ends_at timestamp, after_take bigint, page_size integer)
RETURNS SETOF med_tracker.local_take_history_projection
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$ BEGIN
  IF page_size IS NULL OR page_size NOT BETWEEN 1 AND 100 OR after_take IS NULL OR after_take<0 OR starts_at IS NULL OR ends_at IS NULL OR starts_at>ends_at THEN
    RAISE EXCEPTION 'History page is invalid' USING ERRCODE='23514';
  END IF;
  RETURN QUERY SELECT h.* FROM med_tracker.local_take_history_projection h WHERE h.person_id=subject AND h.taken_at>=starts_at AND h.taken_at<ends_at AND h.id>after_take ORDER BY h.id LIMIT page_size;
END $$;
REVOKE ALL ON FUNCTION med_tracker.local_take_report(bigint,timestamp,timestamp,bigint,integer) FROM PUBLIC, med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.local_take_report(bigint,timestamp,timestamp,bigint,integer) TO med_tracker_app;

CREATE FUNCTION med_tracker.local_historical_subject(subject_ref text) RETURNS TABLE(person_id bigint,portable_id text,name text)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$ SELECT h.person_id,h.person_portable_id,h.subject_name
 FROM med_tracker.local_take_history_projection h
 WHERE h.person_id::text=subject_ref OR h.person_portable_id=subject_ref ORDER BY h.id DESC LIMIT 1 $$;
REVOKE ALL ON FUNCTION med_tracker.local_historical_subject(text) FROM PUBLIC, med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.local_historical_subject(text) TO med_tracker_app;
