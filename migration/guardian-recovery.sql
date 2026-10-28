ALTER TABLE public.guardian_declarations
  ADD COLUMN source text NOT NULL DEFAULT 'initial_attestation',
  ADD COLUMN superseded_at timestamptz,
  ADD COLUMN superseded_by_account_id bigint,
  ADD COLUMN confirmation_token uuid NOT NULL DEFAULT pg_catalog.gen_random_uuid();
ALTER TABLE public.carer_relationships
  ADD COLUMN confirmation_token uuid NOT NULL DEFAULT pg_catalog.gen_random_uuid();

CREATE FUNCTION med_tracker.guardian_recovery_earliest_date()
RETURNS date LANGUAGE sql VOLATILE SET search_path = pg_catalog
AS $body$
  SELECT ((pg_catalog.clock_timestamp() - interval '12 hours') AT TIME ZONE 'UTC')::date
$body$;
ALTER FUNCTION med_tracker.guardian_recovery_earliest_date() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.guardian_recovery_earliest_date() FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.guardian_recovery_earliest_date() TO med_tracker_app;

CREATE FUNCTION med_tracker.guardian_recovery_latest_date()
RETURNS date LANGUAGE sql VOLATILE SET search_path = pg_catalog
AS $body$
  SELECT ((pg_catalog.clock_timestamp() + interval '14 hours') AT TIME ZONE 'UTC')::date
$body$;
ALTER FUNCTION med_tracker.guardian_recovery_latest_date() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.guardian_recovery_latest_date() FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.guardian_recovery_latest_date() TO med_tracker_app;
ALTER TABLE public.guardian_declarations
  ADD CONSTRAINT guardian_declarations_superseding_actor
  FOREIGN KEY (superseded_by_account_id) REFERENCES public.accounts(id) ON DELETE RESTRICT;
ALTER TABLE public.guardian_declarations
  DROP CONSTRAINT guardian_declarations_initial_subject,
  DROP CONSTRAINT guardian_declarations_statement,
  DROP CONSTRAINT guardian_declarations_revision;
ALTER TABLE public.guardian_declarations
  ADD CONSTRAINT guardian_declarations_source_statement CHECK (
    (source = 'initial_attestation' AND statement_version = 1)
    OR (source = 'manager_appointment' AND statement_version = 2)
  ),
  ADD CONSTRAINT guardian_declarations_terminal_state CHECK (
    (withdrawn_at IS NULL AND superseded_at IS NULL
      AND superseded_by_account_id IS NULL AND revision = 1)
    OR (withdrawn_at IS NOT NULL AND superseded_at IS NULL
      AND superseded_by_account_id IS NULL AND revision = 2
      AND withdrawn_at >= declared_at)
    OR (withdrawn_at IS NULL AND superseded_at IS NOT NULL
      AND superseded_by_account_id IS NOT NULL AND revision = 2
      AND superseded_at >= declared_at)
  );
CREATE UNIQUE INDEX guardian_declarations_one_open_subject
  ON public.guardian_declarations(subject_person_id)
  WHERE withdrawn_at IS NULL AND superseded_at IS NULL;
CREATE INDEX guardian_declarations_subject_history
  ON public.guardian_declarations(subject_person_id,declared_at DESC,id DESC);

DROP INDEX public.index_carer_relationships_on_household_carer_patient;
CREATE UNIQUE INDEX index_carer_relationships_on_household_carer_patient
  ON public.carer_relationships(household_id,carer_id,patient_id)
  WHERE guardian_declaration_id IS NULL;
DROP POLICY guardian_declarations_initial_insert ON public.guardian_declarations;
CREATE POLICY guardian_declarations_initial_insert
ON public.guardian_declarations FOR INSERT TO med_tracker_app
WITH CHECK (
  source = 'initial_attestation'
  AND origin_household_id = med_tracker.current_household_id()
  AND statement_version = 1 AND revision = 1
  AND withdrawn_at IS NULL AND superseded_at IS NULL
  AND EXISTS (
    SELECT 1 FROM public.accounts a
    JOIN public.people guardian ON guardian.id = a.person_id
    JOIN public.household_memberships actor ON actor.person_id = guardian.id
    JOIN public.households h ON h.id = actor.household_id
    WHERE a.id = med_tracker.current_account_id()
      AND a.id = guardian_declarations.declared_by_account_id
      AND a.person_id = guardian_declarations.guardian_person_id
      AND a.status = 2 AND a.active
      AND guardian.person_type = 0 AND guardian.has_capacity
      AND actor.id = med_tracker.current_membership_id()
      AND actor.household_id = guardian_declarations.origin_household_id
      AND actor.status = 'active' AND actor.revoked_at IS NULL
      AND h.status = 'active' AND h.lifecycle_state = 'active'
      AND NOT EXISTS (SELECT 1 FROM public.account_lockouts l
        WHERE l.account_id = a.id
          AND l.deadline > pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()))
  )
  AND EXISTS (
    SELECT 1 FROM public.people subject
    JOIN public.household_memberships m ON m.person_id = subject.id
    WHERE subject.id = guardian_declarations.subject_person_id
      AND subject.person_type IN (1,2) AND NOT subject.has_capacity
      AND m.household_id = guardian_declarations.origin_household_id
      AND m.status = 'active' AND m.revoked_at IS NULL
  )
);

CREATE POLICY guardian_recovery_declaration_owner_read
ON public.guardian_declarations FOR SELECT TO med_tracker_owner
USING (subject_person_id = NULLIF(pg_catalog.current_setting(
  'med_tracker.guardian_recovery_subject_id',true),'')::bigint);
CREATE POLICY guardian_recovery_declaration_owner_insert
ON public.guardian_declarations FOR INSERT TO med_tracker_owner
WITH CHECK (source = 'manager_appointment'
  AND subject_person_id = NULLIF(pg_catalog.current_setting(
    'med_tracker.guardian_recovery_subject_id',true),'')::bigint);
CREATE POLICY guardian_recovery_declaration_owner_update
ON public.guardian_declarations FOR UPDATE TO med_tracker_owner
USING (subject_person_id = NULLIF(pg_catalog.current_setting(
  'med_tracker.guardian_recovery_subject_id',true),'')::bigint)
WITH CHECK (subject_person_id = NULLIF(pg_catalog.current_setting(
  'med_tracker.guardian_recovery_subject_id',true),'')::bigint);

CREATE POLICY guardian_recovery_relationship_owner_read
ON public.carer_relationships FOR SELECT TO med_tracker_owner
USING (patient_id = NULLIF(pg_catalog.current_setting(
  'med_tracker.guardian_recovery_subject_id',true),'')::bigint);
CREATE POLICY guardian_recovery_relationship_owner_insert
ON public.carer_relationships FOR INSERT TO med_tracker_owner
WITH CHECK (household_id = med_tracker.current_household_id()
  AND patient_id = NULLIF(pg_catalog.current_setting(
    'med_tracker.guardian_recovery_subject_id',true),'')::bigint);
CREATE POLICY guardian_recovery_relationship_owner_update
ON public.carer_relationships FOR UPDATE TO med_tracker_owner
USING (household_id = med_tracker.current_household_id()
  AND patient_id = NULLIF(pg_catalog.current_setting(
    'med_tracker.guardian_recovery_subject_id',true),'')::bigint)
WITH CHECK (household_id = med_tracker.current_household_id()
  AND patient_id = NULLIF(pg_catalog.current_setting(
    'med_tracker.guardian_recovery_subject_id',true),'')::bigint);

CREATE POLICY guardian_recovery_membership_owner_read
ON public.household_memberships FOR SELECT TO med_tracker_owner
USING (
  id = med_tracker.current_membership_id()
  OR person_id = NULLIF(pg_catalog.current_setting(
    'med_tracker.guardian_recovery_subject_id',true),'')::bigint
  OR person_id = NULLIF(pg_catalog.current_setting(
    'med_tracker.guardian_recovery_guardian_id',true),'')::bigint
  OR person_id IN (
    SELECT d.guardian_person_id FROM public.guardian_declarations d
    WHERE d.subject_person_id = NULLIF(pg_catalog.current_setting(
      'med_tracker.guardian_recovery_subject_id',true),'')::bigint
  )
);
CREATE POLICY guardian_recovery_membership_owner_update
ON public.household_memberships FOR UPDATE TO med_tracker_owner
USING (id = med_tracker.current_membership_id()
  OR person_id = NULLIF(pg_catalog.current_setting(
    'med_tracker.guardian_recovery_subject_id',true),'')::bigint
  OR person_id = NULLIF(pg_catalog.current_setting(
    'med_tracker.guardian_recovery_guardian_id',true),'')::bigint
  OR person_id IN (
    SELECT d.guardian_person_id FROM public.guardian_declarations d
    WHERE d.subject_person_id = NULLIF(pg_catalog.current_setting(
      'med_tracker.guardian_recovery_subject_id',true),'')::bigint
  ))
WITH CHECK (id = med_tracker.current_membership_id()
  OR person_id = NULLIF(pg_catalog.current_setting(
    'med_tracker.guardian_recovery_subject_id',true),'')::bigint
  OR person_id = NULLIF(pg_catalog.current_setting(
    'med_tracker.guardian_recovery_guardian_id',true),'')::bigint
  OR person_id IN (
    SELECT d.guardian_person_id FROM public.guardian_declarations d
    WHERE d.subject_person_id = NULLIF(pg_catalog.current_setting(
      'med_tracker.guardian_recovery_subject_id',true),'')::bigint
  ));
CREATE POLICY guardian_recovery_people_owner_read
ON public.people FOR SELECT TO med_tracker_owner
USING (id = NULLIF(pg_catalog.current_setting(
    'med_tracker.guardian_recovery_subject_id',true),'')::bigint
  OR id = NULLIF(pg_catalog.current_setting(
    'med_tracker.guardian_recovery_guardian_id',true),'')::bigint
  OR id IN (
    SELECT d.guardian_person_id FROM public.guardian_declarations d
    WHERE d.subject_person_id = NULLIF(pg_catalog.current_setting(
      'med_tracker.guardian_recovery_subject_id',true),'')::bigint
  ));
CREATE POLICY guardian_recovery_people_owner_update
ON public.people FOR UPDATE TO med_tracker_owner
USING (id = NULLIF(pg_catalog.current_setting(
    'med_tracker.guardian_recovery_guardian_id',true),'')::bigint
  OR id = NULLIF(pg_catalog.current_setting(
    'med_tracker.guardian_recovery_subject_id',true),'')::bigint)
WITH CHECK (id = NULLIF(pg_catalog.current_setting(
    'med_tracker.guardian_recovery_guardian_id',true),'')::bigint
  OR id = NULLIF(pg_catalog.current_setting(
    'med_tracker.guardian_recovery_subject_id',true),'')::bigint);
CREATE POLICY guardian_recovery_audit_owner_insert
ON public.security_audit_events FOR INSERT TO med_tracker_owner
WITH CHECK (household_id = med_tracker.current_household_id());

CREATE OR REPLACE FUNCTION med_tracker.guardian_declaration_one_way()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog
AS $body$
BEGIN
  IF TG_OP = 'INSERT' THEN
    IF NEW.source = 'manager_appointment' AND current_user <> 'med_tracker_owner' THEN
      RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Recovery command required';
    END IF;
    NEW.declared_at := pg_catalog.clock_timestamp();
    NEW.confirmation_token := pg_catalog.gen_random_uuid();
    RETURN NEW;
  END IF;
  IF TG_OP = 'DELETE' THEN
    RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Guardian declaration provenance is retained';
  END IF;
  IF (pg_catalog.to_jsonb(NEW)-ARRAY['withdrawn_at','superseded_at',
      'superseded_by_account_id','revision','confirmation_token'])
       IS DISTINCT FROM
     (pg_catalog.to_jsonb(OLD)-ARRAY['withdrawn_at','superseded_at',
      'superseded_by_account_id','revision','confirmation_token']) THEN
    RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Guardian declaration identity is immutable';
  END IF;
  IF OLD.withdrawn_at IS NOT NULL OR OLD.superseded_at IS NOT NULL THEN
    IF NEW IS DISTINCT FROM OLD THEN
      RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Guardian declaration terminal state is final';
    END IF;
    RETURN NEW;
  END IF;
  IF NEW.revision <> OLD.revision + 1 THEN
    RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Invalid guardian declaration transition';
  END IF;
  IF NEW.withdrawn_at IS NOT NULL AND NEW.superseded_at IS NULL
      AND NEW.superseded_by_account_id IS NULL THEN
    NEW.withdrawn_at := pg_catalog.clock_timestamp();
  ELSIF NEW.withdrawn_at IS NULL AND NEW.superseded_at IS NOT NULL
      AND NEW.superseded_by_account_id = med_tracker.current_account_id()
      AND current_user = 'med_tracker_owner' THEN
    NEW.superseded_at := pg_catalog.clock_timestamp();
  ELSE
    RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Invalid guardian declaration transition';
  END IF;
  NEW.confirmation_token := pg_catalog.gen_random_uuid();
  RETURN NEW;
END;
$body$;

CREATE FUNCTION med_tracker.guardian_initial_lifetime_guard()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $body$
DECLARE previous_scope text;
BEGIN
  IF NEW.source <> 'initial_attestation' THEN
    RETURN NEW;
  END IF;
  PERFORM pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended('guardian-subject:' || NEW.subject_person_id::text,0));
  previous_scope := pg_catalog.current_setting('med_tracker.guardian_recovery_subject_id',true);
  PERFORM pg_catalog.set_config('med_tracker.guardian_recovery_subject_id',
    NEW.subject_person_id::text,true);
  IF EXISTS (SELECT 1 FROM public.guardian_declarations d
             WHERE d.subject_person_id = NEW.subject_person_id) THEN
    RAISE EXCEPTION USING ERRCODE = '23505', MESSAGE = 'Initial guardian declaration already exists';
  END IF;
  PERFORM pg_catalog.set_config('med_tracker.guardian_recovery_subject_id',
    COALESCE(previous_scope,''),true);
  RETURN NEW;
END;
$body$;
ALTER FUNCTION med_tracker.guardian_initial_lifetime_guard() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.guardian_initial_lifetime_guard() FROM PUBLIC,med_tracker_app;
CREATE TRIGGER guardian_declarations_initial_lifetime
BEFORE INSERT ON public.guardian_declarations
FOR EACH ROW EXECUTE FUNCTION med_tracker.guardian_initial_lifetime_guard();

CREATE FUNCTION med_tracker.guardian_recovery_link_guard()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog
AS $body$
DECLARE initial_declaration public.guardian_declarations%ROWTYPE;
BEGIN
  IF TG_OP = 'DELETE' THEN
    IF OLD.guardian_declaration_id IS NOT NULL THEN
      RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Guardian relationship provenance is retained';
    END IF;
    RETURN OLD;
  END IF;
  IF TG_OP = 'UPDATE' THEN
    IF OLD.confirmation_token IS DISTINCT FROM NEW.confirmation_token THEN
      RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Guardian relationship confirmation token is managed';
    END IF;
    IF OLD.guardian_declaration_id IS DISTINCT FROM NEW.guardian_declaration_id
       OR (OLD.guardian_declaration_id IS NOT NULL AND
         (OLD.carer_id IS DISTINCT FROM NEW.carer_id
          OR OLD.patient_id IS DISTINCT FROM NEW.patient_id
          OR OLD.household_id IS DISTINCT FROM NEW.household_id)) THEN
      RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Guardian relationship identity is immutable';
    END IF;
    IF OLD.guardian_declaration_id IS NOT NULL AND NOT OLD.active AND NEW.active
       AND current_user <> 'med_tracker_owner' THEN
      RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Recovery command required';
    END IF;
    IF OLD.active IS DISTINCT FROM NEW.active THEN
      NEW.confirmation_token := pg_catalog.gen_random_uuid();
    END IF;
  END IF;
  IF TG_OP = 'INSERT' AND NEW.guardian_declaration_id IS NOT NULL
     AND current_user <> 'med_tracker_owner' THEN
    -- Match withdrawal and recovery's account-before-declaration lock order.
    PERFORM a.id FROM public.accounts a
    JOIN public.household_memberships actor ON actor.person_id = a.person_id
    JOIN public.households h ON h.id = actor.household_id
    WHERE a.id = med_tracker.current_account_id() AND a.person_id = NEW.carer_id
      AND a.status = 2 AND a.active
      AND actor.id = med_tracker.current_membership_id()
      AND actor.household_id = NEW.household_id
      AND actor.household_id = med_tracker.current_household_id()
      AND actor.status = 'active' AND actor.revoked_at IS NULL
      AND h.status = 'active' AND h.lifecycle_state = 'active'
      AND NOT EXISTS (SELECT 1 FROM public.account_lockouts l WHERE l.account_id = a.id
        AND l.deadline > pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()))
    FOR SHARE OF a;
    IF NOT FOUND THEN
      RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Recovery command required';
    END IF;
    SELECT d.* INTO initial_declaration FROM public.guardian_declarations d
    WHERE d.id = NEW.guardian_declaration_id FOR SHARE OF d;
    IF NOT FOUND THEN
      RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Recovery command required';
    END IF;
    IF initial_declaration.source IS DISTINCT FROM 'initial_attestation'
       OR initial_declaration.origin_household_id IS DISTINCT FROM NEW.household_id
       OR initial_declaration.subject_person_id IS DISTINCT FROM NEW.patient_id
       OR initial_declaration.guardian_person_id IS DISTINCT FROM NEW.carer_id
       OR initial_declaration.declared_by_account_id IS DISTINCT FROM med_tracker.current_account_id()
       OR initial_declaration.withdrawn_at IS NOT NULL
       OR initial_declaration.superseded_at IS NOT NULL
       OR EXISTS (SELECT 1 FROM public.carer_relationships c
         WHERE c.household_id = NEW.household_id
           AND c.guardian_declaration_id = NEW.guardian_declaration_id) THEN
      RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Recovery command required';
    END IF;
  END IF;
  IF TG_OP = 'INSERT' THEN
    NEW.confirmation_token := pg_catalog.gen_random_uuid();
  END IF;
  RETURN NEW;
END;
$body$;
ALTER FUNCTION med_tracker.guardian_recovery_link_guard() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.guardian_recovery_link_guard() FROM PUBLIC,med_tracker_app;
CREATE TRIGGER guardian_recovery_link_guard
BEFORE INSERT OR UPDATE OR DELETE ON public.carer_relationships
FOR EACH ROW EXECUTE FUNCTION med_tracker.guardian_recovery_link_guard();

CREATE FUNCTION med_tracker.guardian_recovery_snapshot(p_subject bigint)
RETURNS TABLE(snapshot_token uuid, snapshot_revision bigint, link_token uuid, has_effective_guardian boolean)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = pg_catalog
AS $body$
BEGIN
  IF p_subject IS NULL OR p_subject <= 0 THEN
    RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Guardian recovery unavailable';
  END IF;
  PERFORM pg_catalog.set_config('med_tracker.guardian_recovery_subject_id',
    p_subject::text,true);
  IF NOT EXISTS (
    SELECT 1 FROM public.household_memberships actor
    JOIN public.accounts a ON a.person_id = actor.person_id
    JOIN public.households h ON h.id = actor.household_id
    JOIN public.household_memberships subject_member
      ON subject_member.household_id = h.id AND subject_member.person_id = p_subject
    JOIN public.people subject ON subject.id = p_subject
    WHERE actor.id = med_tracker.current_membership_id()
      AND actor.household_id = med_tracker.current_household_id()
      AND actor.role IN ('owner','administrator')
      AND actor.status = 'active' AND actor.revoked_at IS NULL
      AND a.id = med_tracker.current_account_id() AND a.status = 2
      AND a.active AND h.status = 'active' AND h.lifecycle_state = 'active'
      AND subject_member.status = 'active' AND subject_member.revoked_at IS NULL
      AND subject.person_type = 1 AND NOT subject.has_capacity
      AND subject.date_of_birth IS NOT NULL
      AND subject.date_of_birth <= med_tracker.guardian_recovery_earliest_date()
      AND subject.date_of_birth >
          (med_tracker.guardian_recovery_latest_date() - interval '18 years')::date
      AND NOT EXISTS (SELECT 1 FROM public.account_lockouts l
        WHERE l.account_id = a.id
          AND l.deadline > pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()))
  ) THEN
    RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Guardian recovery unavailable';
  END IF;
  SELECT d.confirmation_token,d.revision
    INTO snapshot_token,snapshot_revision
    FROM public.guardian_declarations d
    WHERE d.subject_person_id = p_subject
    ORDER BY d.declared_at DESC,d.id DESC LIMIT 1;
  IF NOT FOUND THEN
    snapshot_token := '00000000-0000-0000-0000-000000000000'::uuid;
    snapshot_revision := 0;
  END IF;
  SELECT c.confirmation_token INTO link_token
    FROM public.carer_relationships c
    JOIN public.guardian_declarations d ON d.id=c.guardian_declaration_id
    WHERE d.subject_person_id=p_subject AND d.withdrawn_at IS NULL
      AND d.superseded_at IS NULL AND c.household_id=med_tracker.current_household_id()
    ORDER BY d.declared_at DESC,d.id DESC,c.id DESC LIMIT 1;
  IF NOT FOUND THEN
    link_token := '00000000-0000-0000-0000-000000000000'::uuid;
  END IF;
  SELECT EXISTS (
    SELECT 1 FROM public.guardian_declarations d
    JOIN public.carer_relationships c
      ON c.guardian_declaration_id = d.id
      AND c.carer_id = d.guardian_person_id
      AND c.patient_id = d.subject_person_id
    JOIN public.households h ON h.id = c.household_id
    JOIN public.household_memberships subject_member
      ON subject_member.household_id = h.id
      AND subject_member.person_id = d.subject_person_id
    JOIN public.household_memberships guardian_member
      ON guardian_member.household_id = h.id
      AND guardian_member.person_id = d.guardian_person_id
    JOIN public.people guardian ON guardian.id = d.guardian_person_id
    JOIN public.accounts a ON a.person_id = guardian.id
    WHERE d.subject_person_id = p_subject
      AND d.withdrawn_at IS NULL AND d.superseded_at IS NULL
      AND c.active AND h.status = 'active' AND h.lifecycle_state = 'active'
      AND subject_member.status = 'active' AND subject_member.revoked_at IS NULL
      AND guardian_member.status = 'active' AND guardian_member.revoked_at IS NULL
      AND guardian.person_type = 0 AND guardian.has_capacity
      AND (guardian.date_of_birth IS NULL OR guardian.date_of_birth <=
        (med_tracker.guardian_recovery_latest_date() - interval '18 years')::date)
      AND a.status = 2 AND a.active
  ) INTO has_effective_guardian;
  RETURN NEXT;
END;
$body$;
ALTER FUNCTION med_tracker.guardian_recovery_snapshot(bigint) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.guardian_recovery_snapshot(bigint) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.guardian_recovery_snapshot(bigint) TO med_tracker_app;

CREATE FUNCTION med_tracker.guardian_recovery_command(
  p_subject bigint, p_guardian bigint, p_expected_token uuid,
  p_expected_revision bigint, p_expected_link_token uuid,
  p_restore_declaration bigint, p_restore_link bigint,
  p_reason text, p_request_id text)
RETURNS TABLE(outcome text, declaration_id bigint, relationship_id bigint)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = pg_catalog
AS $body$
DECLARE
  latest_token uuid;
  latest_revision bigint;
  latest_link_token uuid;
  guarded boolean;
  candidate_eligible boolean;
  old_declaration bigint;
  old_guardian bigint;
  guardian_membership bigint;
  actor_account bigint;
  subject_name text;
  event_metadata jsonb;
  event_context jsonb;
  timestamp_utc timestamp;
BEGIN
  IF p_subject IS NULL OR p_subject <= 0 OR p_guardian IS NULL OR p_guardian <= 0
     OR p_expected_token IS NULL OR p_expected_revision IS NULL
     OR p_expected_link_token IS NULL
     OR (p_restore_declaration IS NULL) <> (p_restore_link IS NULL)
     OR pg_catalog.length(pg_catalog.btrim(COALESCE(p_reason,''))) NOT BETWEEN 10 AND 500
     OR pg_catalog.length(COALESCE(p_request_id,'')) NOT BETWEEN 1 AND 128 THEN
    RETURN QUERY SELECT 'invalid'::text,NULL::bigint,NULL::bigint;
    RETURN;
  END IF;
  PERFORM pg_catalog.set_config('med_tracker.guardian_recovery_subject_id',
    p_subject::text,true);
  PERFORM pg_catalog.set_config('med_tracker.guardian_recovery_guardian_id',
    p_guardian::text,true);
  -- This command independently verifies the actor and local minor before any lock or write.
  IF NOT EXISTS (
    SELECT 1 FROM public.household_memberships actor
    JOIN public.accounts a ON a.person_id = actor.person_id
    JOIN public.households h ON h.id = actor.household_id
    JOIN public.household_memberships subject_member
      ON subject_member.household_id = h.id AND subject_member.person_id = p_subject
    JOIN public.people subject ON subject.id = p_subject
    WHERE actor.id = med_tracker.current_membership_id()
      AND actor.household_id = med_tracker.current_household_id()
      AND actor.role IN ('owner','administrator')
      AND actor.status = 'active' AND actor.revoked_at IS NULL
      AND a.id = med_tracker.current_account_id() AND a.status = 2
      AND a.active AND h.status = 'active' AND h.lifecycle_state = 'active'
      AND subject_member.status = 'active' AND subject_member.revoked_at IS NULL
      AND subject.person_type = 1 AND NOT subject.has_capacity
      AND subject.date_of_birth IS NOT NULL
      AND subject.date_of_birth <= med_tracker.guardian_recovery_earliest_date()
      AND subject.date_of_birth >
          (med_tracker.guardian_recovery_latest_date() - interval '18 years')::date
      AND NOT EXISTS (SELECT 1 FROM public.account_lockouts l
        WHERE l.account_id = a.id
          AND l.deadline > pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()))
  ) THEN
    RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Guardian recovery unavailable';
  END IF;
  -- Own withdrawal locks its account before its declaration; use that same order.
  PERFORM a.id FROM public.accounts a
    WHERE a.id = med_tracker.current_account_id() OR a.person_id = p_guardian
    ORDER BY a.id FOR SHARE OF a;
  PERFORM 1 FROM public.households h
    WHERE h.id = med_tracker.current_household_id() FOR UPDATE;
  PERFORM pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended('guardian-subject:' || p_subject::text,0));
  PERFORM d.id FROM public.guardian_declarations d
    WHERE d.subject_person_id = p_subject
    ORDER BY d.declared_at DESC,d.id DESC LIMIT 1 FOR UPDATE OF d;
  SELECT a.id,subject.name INTO actor_account,subject_name
    FROM public.household_memberships actor
    JOIN public.accounts a ON a.person_id = actor.person_id
    JOIN public.households h ON h.id = actor.household_id
    JOIN public.household_memberships subject_member
      ON subject_member.household_id = h.id AND subject_member.person_id = p_subject
    JOIN public.people subject ON subject.id = p_subject
    WHERE actor.id = med_tracker.current_membership_id()
      AND actor.household_id = med_tracker.current_household_id()
      AND actor.role IN ('owner','administrator')
      AND actor.status = 'active' AND actor.revoked_at IS NULL
      AND a.id = med_tracker.current_account_id() AND a.status = 2
      AND a.active AND h.status = 'active' AND h.lifecycle_state = 'active'
      AND subject_member.status = 'active' AND subject_member.revoked_at IS NULL
      AND subject.person_type = 1 AND NOT subject.has_capacity
      AND subject.date_of_birth IS NOT NULL
      AND subject.date_of_birth <= med_tracker.guardian_recovery_earliest_date()
      AND subject.date_of_birth >
          (med_tracker.guardian_recovery_latest_date() - interval '18 years')::date
      AND NOT EXISTS (SELECT 1 FROM public.account_lockouts l
        WHERE l.account_id = a.id
          AND l.deadline > pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()))
    FOR SHARE OF actor,a,subject_member,subject;
  IF NOT FOUND THEN
    RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Guardian recovery unavailable';
  END IF;
  SELECT m.id INTO guardian_membership
    FROM public.household_memberships m
    JOIN public.people p ON p.id = m.person_id
    JOIN public.accounts a ON a.person_id = p.id
    WHERE m.person_id = p_guardian
      AND m.household_id = med_tracker.current_household_id()
      AND m.status = 'active' AND m.revoked_at IS NULL
      AND p.person_type = 0 AND p.has_capacity
      AND (p.date_of_birth IS NULL OR p.date_of_birth <=
        (med_tracker.guardian_recovery_earliest_date() - interval '18 years')::date)
      AND a.status = 2 AND a.active
      AND NOT EXISTS (SELECT 1 FROM public.account_lockouts l
        WHERE l.account_id = a.id
          AND l.deadline > pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()))
    FOR SHARE OF m,p,a;
  IF NOT FOUND THEN
    RETURN QUERY SELECT 'invalid_guardian'::text,NULL::bigint,NULL::bigint;
    RETURN;
  END IF;
  PERFORM c.id FROM public.carer_relationships c
    JOIN public.guardian_declarations d ON d.id = c.guardian_declaration_id
    WHERE d.subject_person_id = p_subject
      AND d.withdrawn_at IS NULL AND d.superseded_at IS NULL
    FOR SHARE OF c;
  SELECT s.snapshot_token,s.snapshot_revision,s.link_token,s.has_effective_guardian,
      EXISTS (
        SELECT 1 FROM public.household_memberships m
        JOIN public.people p ON p.id = m.person_id
        JOIN public.accounts a ON a.person_id = p.id
        WHERE m.person_id = p_guardian
          AND m.household_id = med_tracker.current_household_id()
          AND m.status = 'active' AND m.revoked_at IS NULL
          AND p.person_type = 0 AND p.has_capacity
          AND (p.date_of_birth IS NULL OR p.date_of_birth <=
            (med_tracker.guardian_recovery_earliest_date() - interval '18 years')::date)
          AND a.status = 2 AND a.active
          AND NOT EXISTS (SELECT 1 FROM public.account_lockouts l
            WHERE l.account_id = a.id
              AND l.deadline > pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()))
      )
    INTO latest_token,latest_revision,latest_link_token,guarded,candidate_eligible
    FROM med_tracker.guardian_recovery_snapshot(p_subject) s;
  IF NOT candidate_eligible THEN
    RETURN QUERY SELECT 'invalid_guardian'::text,NULL::bigint,NULL::bigint;
    RETURN;
  END IF;
  IF p_expected_token <> latest_token OR p_expected_revision <> latest_revision
     OR p_expected_link_token <> latest_link_token THEN
    RETURN QUERY SELECT 'stale'::text,NULL::bigint,NULL::bigint;
    RETURN;
  END IF;
  IF guarded THEN
    RETURN QUERY SELECT 'already_guarded'::text,NULL::bigint,NULL::bigint;
    RETURN;
  END IF;
  IF p_restore_declaration IS NOT NULL THEN
    SELECT d.id,c.id INTO declaration_id,relationship_id
      FROM public.guardian_declarations d
      JOIN public.carer_relationships c ON c.guardian_declaration_id = d.id
      WHERE d.id = p_restore_declaration AND c.id = p_restore_link
        AND d.subject_person_id = p_subject AND d.guardian_person_id = p_guardian
        AND d.withdrawn_at IS NULL AND d.superseded_at IS NULL
        AND c.household_id = med_tracker.current_household_id()
        AND c.carer_id = p_guardian AND c.patient_id = p_subject AND NOT c.active
      FOR UPDATE OF d,c;
    IF NOT FOUND THEN
      RETURN QUERY SELECT 'invalid_restore'::text,NULL::bigint,NULL::bigint;
      RETURN;
    END IF;
    UPDATE public.carer_relationships
      SET active = true,updated_at = pg_catalog.timezone('UTC',pg_catalog.clock_timestamp())
      WHERE id = relationship_id;
    outcome := 'restored';
  ELSE
    IF EXISTS (
      SELECT 1 FROM public.guardian_declarations d
      JOIN public.carer_relationships c ON c.guardian_declaration_id = d.id
      JOIN public.household_memberships m
        ON m.person_id = d.guardian_person_id AND m.household_id = c.household_id
      JOIN public.people p ON p.id = d.guardian_person_id
      JOIN public.accounts a ON a.person_id = p.id
      WHERE d.subject_person_id = p_subject
        AND d.guardian_person_id = p_guardian
        AND d.withdrawn_at IS NULL AND d.superseded_at IS NULL
        AND c.household_id = med_tracker.current_household_id() AND NOT c.active
        AND m.status = 'active' AND m.revoked_at IS NULL
        AND p.person_type = 0 AND p.has_capacity AND a.status = 2 AND a.active
        AND (p.date_of_birth IS NULL OR p.date_of_birth <=
          (med_tracker.guardian_recovery_earliest_date() - interval '18 years')::date)
    ) THEN
      RETURN QUERY SELECT 'restore_available'::text,NULL::bigint,NULL::bigint;
      RETURN;
    END IF;
    SELECT d.id,d.guardian_person_id INTO old_declaration,old_guardian
      FROM public.guardian_declarations d
      WHERE d.subject_person_id = p_subject
        AND d.withdrawn_at IS NULL AND d.superseded_at IS NULL
      FOR UPDATE;
    IF FOUND THEN
      UPDATE public.guardian_declarations
        SET superseded_at = pg_catalog.clock_timestamp(),
            superseded_by_account_id = med_tracker.current_account_id(),
            revision = revision + 1
        WHERE id = old_declaration;
    END IF;
    INSERT INTO public.guardian_declarations
      (subject_person_id,guardian_person_id,origin_household_id,
       declared_by_account_id,statement_version,source,subject_name_at_declaration)
      VALUES (p_subject,p_guardian,med_tracker.current_household_id(),
              med_tracker.current_account_id(),2,'manager_appointment',subject_name)
      RETURNING id INTO declaration_id;
    INSERT INTO public.carer_relationships
      (household_id,carer_id,patient_id,guardian_declaration_id,
       relationship_type,active,created_at,updated_at)
      VALUES (med_tracker.current_household_id(),p_guardian,p_subject,declaration_id,
              'family_member',true,
              pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()),
              pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()))
      RETURNING id INTO relationship_id;
    outcome := 'appointed';
  END IF;
  UPDATE public.household_memberships m
    SET permissions_version = m.permissions_version + 1,
        updated_at = pg_catalog.timezone('UTC',pg_catalog.clock_timestamp())
    WHERE m.person_id = p_guardian OR (old_guardian IS NOT NULL AND m.person_id = old_guardian);
  timestamp_utc := pg_catalog.timezone('UTC',pg_catalog.clock_timestamp());
  event_metadata := pg_catalog.jsonb_build_object(
    'subject_person_id',p_subject,'guardian_person_id',p_guardian,
    'action',outcome,'reason',pg_catalog.btrim(p_reason));
  event_context := pg_catalog.jsonb_build_object(
    'actor_account_id',med_tracker.current_account_id(),
    'actor_person_id',(SELECT a.person_id FROM public.accounts a WHERE a.id=actor_account),
    'actor_membership_id',med_tracker.current_membership_id(),
    'household_id',med_tracker.current_household_id(),
    'request_id',p_request_id,'policy_class','GuardianRecovery',
    'policy_query',outcome);
  INSERT INTO public.versions
    (item_type,item_id,event,object,object_changes,whodunnit,request_id,
     household_id,actor_membership_id,audit_context,created_at)
    VALUES ('GuardianDeclaration',declaration_id,outcome,NULL,
      event_metadata::text,actor_account::text,p_request_id,
      med_tracker.current_household_id(),med_tracker.current_membership_id(),
      event_context,timestamp_utc);
  INSERT INTO public.versions
    (item_type,item_id,event,object,object_changes,whodunnit,request_id,
     household_id,actor_membership_id,audit_context,created_at)
    VALUES ('CarerRelationship',relationship_id,outcome,NULL,
      event_metadata::text,actor_account::text,p_request_id,
      med_tracker.current_household_id(),med_tracker.current_membership_id(),
      event_context,timestamp_utc);
  INSERT INTO public.security_audit_events
    (household_id,actor_account_id,actor_membership_id,event_type,
     request_id,metadata,audit_context,created_at,updated_at)
    VALUES (med_tracker.current_household_id(),med_tracker.current_account_id(),
      med_tracker.current_membership_id(),'guardian_recovery.' || outcome,
      p_request_id,event_metadata,event_context,timestamp_utc,timestamp_utc);
  RETURN NEXT;
END;
$body$;
ALTER FUNCTION med_tracker.guardian_recovery_command(bigint,bigint,uuid,bigint,uuid,bigint,bigint,text,text)
  OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.guardian_recovery_command(bigint,bigint,uuid,bigint,uuid,bigint,bigint,text,text)
  FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.guardian_recovery_command(bigint,bigint,uuid,bigint,uuid,bigint,bigint,text,text)
  TO med_tracker_app;
