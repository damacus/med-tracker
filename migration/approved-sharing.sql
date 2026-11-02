ALTER TABLE public.person_household_approvals OWNER TO med_tracker_owner;
ALTER TABLE public.person_household_approvals ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.person_household_approvals FORCE ROW LEVEL SECURITY;
REVOKE ALL ON public.person_household_approvals FROM PUBLIC,med_tracker_app;
GRANT SELECT,INSERT ON public.person_household_approvals TO med_tracker_app;
GRANT UPDATE(ended_at) ON public.person_household_approvals TO med_tracker_app;
GRANT USAGE ON SEQUENCE public.person_household_approvals_id_seq TO med_tracker_app;
CREATE UNIQUE INDEX sharing_one_current_approval
ON public.person_household_approvals(person_id,household_id) WHERE ended_at IS NULL;
CREATE UNIQUE INDEX sharing_invitation_once
ON public.person_household_approvals(invitation_id);
CREATE POLICY sharing_accepted_recipient ON public.household_invitations FOR SELECT TO med_tracker_app,med_tracker_owner
USING (token_digest=med_tracker.current_invitation_token_digest() AND accepted_at IS NOT NULL
 AND lower(email)=(SELECT lower(email) FROM public.accounts WHERE id=med_tracker.current_account_id() AND status=2 AND active));

CREATE POLICY sharing_subject_people ON public.people FOR SELECT TO med_tracker_owner
USING (id=NULLIF(current_setting('med_tracker.sharing_subject_id',true),'')::bigint
 OR id=(SELECT person_id FROM public.accounts WHERE id=med_tracker.current_account_id())
 OR id IN (SELECT guardian_person_id FROM public.guardian_declarations WHERE subject_person_id=NULLIF(current_setting('med_tracker.sharing_subject_id',true),'')::bigint));
CREATE POLICY sharing_subject_memberships ON public.household_memberships FOR SELECT TO med_tracker_owner
USING (person_id=NULLIF(current_setting('med_tracker.sharing_subject_id',true),'')::bigint
 OR person_id=(SELECT person_id FROM public.accounts WHERE id=med_tracker.current_account_id())
 OR person_id IN (SELECT guardian_person_id FROM public.guardian_declarations WHERE subject_person_id=NULLIF(current_setting('med_tracker.sharing_subject_id',true),'')::bigint));
CREATE POLICY sharing_subject_declarations ON public.guardian_declarations FOR SELECT TO med_tracker_owner
USING (subject_person_id=NULLIF(current_setting('med_tracker.sharing_subject_id',true),'')::bigint);
CREATE POLICY sharing_subject_relationships ON public.carer_relationships FOR SELECT TO med_tracker_owner
USING (patient_id=NULLIF(current_setting('med_tracker.sharing_subject_id',true),'')::bigint);
CREATE POLICY sharing_subject_homes ON public.households FOR SELECT TO med_tracker_owner
USING (id IN (SELECT household_id FROM public.household_memberships
 WHERE person_id=NULLIF(current_setting('med_tracker.sharing_subject_id',true),'')::bigint));

CREATE FUNCTION med_tracker.sharing_declaration_eligible(p_declaration bigint,p_subject bigint) RETURNS boolean
LANGUAGE sql VOLATILE SET search_path=pg_catalog AS $body$
 SELECT EXISTS(SELECT 1 FROM public.guardian_declarations d
 JOIN public.people guardian ON guardian.id=d.guardian_person_id
 JOIN public.people subject ON subject.id=d.subject_person_id
 JOIN public.accounts a ON a.person_id=guardian.id
 JOIN public.carer_relationships c ON c.guardian_declaration_id=d.id AND c.carer_id=guardian.id AND c.patient_id=subject.id AND c.active
 JOIN public.household_memberships gm ON gm.person_id=guardian.id AND gm.household_id=c.household_id
 JOIN public.household_memberships sm ON sm.person_id=subject.id AND sm.household_id=c.household_id
 JOIN public.households h ON h.id=c.household_id
 WHERE d.id=p_declaration AND d.subject_person_id=p_subject AND d.withdrawn_at IS NULL AND d.superseded_at IS NULL
 AND a.status=2 AND a.active AND gm.status='active' AND gm.revoked_at IS NULL AND sm.status='active' AND sm.revoked_at IS NULL
 AND h.status='active' AND h.lifecycle_state='active'
 AND guardian.person_type=0 AND guardian.has_capacity
 AND (guardian.date_of_birth IS NULL OR guardian.date_of_birth<=med_tracker.guardian_recovery_earliest_date()-interval '18 years')
 AND NOT subject.has_capacity AND subject.date_of_birth IS NOT NULL
 AND ((subject.person_type=1 AND subject.date_of_birth>med_tracker.guardian_recovery_latest_date()-interval '18 years'
  AND subject.date_of_birth<=med_tracker.guardian_recovery_earliest_date())
 OR (subject.person_type=2 AND subject.date_of_birth<=med_tracker.guardian_recovery_earliest_date()-interval '18 years')))
$body$;
ALTER FUNCTION med_tracker.sharing_declaration_eligible(bigint,bigint) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.sharing_declaration_eligible(bigint,bigint) FROM PUBLIC,med_tracker_app;

CREATE FUNCTION med_tracker.sharing_guardian(p_subject bigint) RETURNS bigint
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=pg_catalog AS $body$
DECLARE previous_subject text; declaration bigint;
BEGIN
 previous_subject:=current_setting('med_tracker.sharing_subject_id',true);
 PERFORM set_config('med_tracker.sharing_subject_id',p_subject::text,true);
 SELECT d.id INTO declaration FROM public.guardian_declarations d
 JOIN public.accounts a ON a.person_id=d.guardian_person_id
 JOIN public.people guardian ON guardian.id=a.person_id
 JOIN public.people subject ON subject.id=d.subject_person_id
 JOIN public.carer_relationships c ON c.guardian_declaration_id=d.id
  AND c.carer_id=d.guardian_person_id AND c.patient_id=d.subject_person_id AND c.active
 JOIN public.household_memberships gm ON gm.person_id=guardian.id AND gm.household_id=c.household_id
 JOIN public.household_memberships sm ON sm.person_id=subject.id AND sm.household_id=c.household_id
 JOIN public.households h ON h.id=c.household_id
 WHERE a.id=med_tracker.current_account_id() AND a.status=2 AND a.active
  AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>clock_timestamp())
  AND d.subject_person_id=p_subject AND d.withdrawn_at IS NULL AND d.superseded_at IS NULL
  AND med_tracker.sharing_declaration_eligible(d.id,p_subject)
  AND gm.status='active' AND gm.revoked_at IS NULL AND sm.status='active' AND sm.revoked_at IS NULL
  AND h.status='active' AND h.lifecycle_state='active'
  AND guardian.person_type=0 AND guardian.has_capacity
  AND (guardian.date_of_birth IS NULL OR guardian.date_of_birth<=med_tracker.guardian_recovery_earliest_date()-interval '18 years')
  AND NOT subject.has_capacity AND subject.date_of_birth IS NOT NULL
  AND ((subject.person_type=1 AND subject.date_of_birth>med_tracker.guardian_recovery_latest_date()-interval '18 years'
    AND subject.date_of_birth<=med_tracker.guardian_recovery_earliest_date())
   OR (subject.person_type=2 AND subject.date_of_birth<=med_tracker.guardian_recovery_earliest_date()-interval '18 years'))
 ORDER BY d.id LIMIT 1;
 PERFORM set_config('med_tracker.sharing_subject_id',COALESCE(previous_subject,''),true);
 RETURN declaration;
END $body$;
ALTER FUNCTION med_tracker.sharing_guardian(bigint) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.sharing_guardian(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION med_tracker.sharing_guardian(bigint) TO med_tracker_app;

CREATE FUNCTION med_tracker.sharing_clinical_authority(p_subject bigint) RETURNS boolean
LANGUAGE sql VOLATILE SET search_path=pg_catalog AS $body$
 SELECT EXISTS(SELECT 1 FROM public.accounts a WHERE a.id=med_tracker.current_account_id()
  AND a.person_id=p_subject AND a.status=2 AND a.active
  AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>clock_timestamp()))
 OR med_tracker.sharing_guardian(p_subject) IS NOT NULL
$body$;
ALTER FUNCTION med_tracker.sharing_clinical_authority(bigint) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.sharing_clinical_authority(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION med_tracker.sharing_clinical_authority(bigint) TO med_tracker_app;

CREATE FUNCTION med_tracker.sharing_household_actor(p_household bigint,p_management boolean) RETURNS boolean
LANGUAGE sql VOLATILE SET search_path=pg_catalog AS $body$
 SELECT EXISTS(SELECT 1 FROM public.household_memberships member
 JOIN public.accounts actor ON actor.person_id=member.person_id
 JOIN public.households home ON home.id=member.household_id
 WHERE member.id=med_tracker.current_membership_id() AND actor.id=med_tracker.current_account_id()
  AND member.household_id=p_household AND p_household=med_tracker.current_household_id()
  AND member.status='active' AND member.revoked_at IS NULL AND actor.status=2 AND actor.active
  AND home.status='active' AND home.lifecycle_state='active'
  AND (NOT p_management OR member.role IN ('owner','administrator'))
  AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=actor.id AND l.deadline>clock_timestamp()))
$body$;
ALTER FUNCTION med_tracker.sharing_household_actor(bigint,boolean) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.sharing_household_actor(bigint,boolean) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION med_tracker.sharing_household_actor(bigint,boolean) TO med_tracker_app;

CREATE POLICY sharing_approval_read ON public.person_household_approvals FOR SELECT
USING (med_tracker.sharing_household_actor(household_id,false) OR med_tracker.sharing_clinical_authority(person_id));
CREATE POLICY sharing_approval_insert ON public.person_household_approvals FOR INSERT
WITH CHECK (household_id=med_tracker.current_household_id()
 AND approved_by_account_id=med_tracker.current_account_id()
 AND approved_by_person_id=(SELECT person_id FROM public.accounts WHERE id=med_tracker.current_account_id())
 AND med_tracker.sharing_clinical_authority(person_id));
CREATE POLICY sharing_approval_end ON public.person_household_approvals FOR UPDATE
USING (med_tracker.sharing_household_actor(household_id,true) OR med_tracker.sharing_clinical_authority(person_id))
WITH CHECK (med_tracker.sharing_household_actor(household_id,true) OR med_tracker.sharing_clinical_authority(person_id));

CREATE FUNCTION med_tracker.sharing_approval_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $body$
BEGIN
 IF TG_OP='UPDATE' THEN
  IF OLD.ended_at IS NOT NULL OR NEW.ended_at IS NULL
   OR (to_jsonb(NEW)-'ended_at') IS DISTINCT FROM (to_jsonb(OLD)-'ended_at') THEN
   RAISE EXCEPTION 'Sharing approval is terminal' USING ERRCODE='42501';
  END IF;
 ELSE
  IF NEW.statement_version<>1 OR NEW.ended_at IS NOT NULL
   OR NOT med_tracker.sharing_clinical_authority(NEW.person_id)
   OR (NEW.person_id<>NEW.approved_by_person_id AND NEW.guardian_declaration_id IS DISTINCT FROM med_tracker.sharing_guardian(NEW.person_id))
   OR NOT EXISTS(SELECT 1 FROM public.household_invitations i
    JOIN public.household_memberships inviter ON inviter.id=i.invited_by_membership_id AND inviter.household_id=i.household_id
    JOIN public.accounts actor ON actor.id=NEW.approved_by_account_id
    JOIN public.accounts issuer ON issuer.person_id=inviter.person_id
    WHERE i.id=NEW.invitation_id AND i.household_id=NEW.household_id
     AND lower(i.email)=lower(actor.email) AND actor.status=2 AND actor.active
     AND issuer.status=2 AND issuer.active AND inviter.status='active' AND inviter.revoked_at IS NULL
     AND inviter.role IN ('owner','administrator') AND i.revoked_at IS NULL
     AND i.expires_at>timezone('UTC',clock_timestamp())
     AND i.dependent_sharing=(NEW.person_id<>NEW.approved_by_person_id)) THEN
   RAISE EXCEPTION 'Sharing consent unavailable' USING ERRCODE='42501';
  END IF;
 END IF;
 RETURN NEW;
END $body$;
ALTER FUNCTION med_tracker.sharing_approval_guard() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.sharing_approval_guard() FROM PUBLIC,med_tracker_app;
CREATE TRIGGER sharing_approval_guard BEFORE INSERT OR UPDATE ON public.person_household_approvals
FOR EACH ROW EXECUTE FUNCTION med_tracker.sharing_approval_guard();

CREATE FUNCTION med_tracker.sharing_current_member(p_subject bigint) RETURNS boolean
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=pg_catalog AS $body$
DECLARE previous_subject text; permitted boolean;
BEGIN
 previous_subject:=current_setting('med_tracker.sharing_subject_id',true);
 PERFORM set_config('med_tracker.sharing_subject_id',p_subject::text,true);
 SELECT EXISTS(SELECT 1 FROM public.person_household_approvals approval
 JOIN public.household_memberships subject ON subject.person_id=approval.person_id AND subject.household_id=approval.household_id
 JOIN public.household_memberships actor ON actor.id=med_tracker.current_membership_id() AND actor.household_id=approval.household_id
 JOIN public.accounts account ON account.person_id=actor.person_id AND account.id=med_tracker.current_account_id()
 JOIN public.households h ON h.id=approval.household_id
 WHERE approval.person_id=p_subject AND approval.household_id=med_tracker.current_household_id() AND approval.ended_at IS NULL
  AND subject.status='active' AND subject.revoked_at IS NULL AND actor.status='active' AND actor.revoked_at IS NULL
  AND account.status=2 AND account.active AND h.status='active' AND h.lifecycle_state='active'
  AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=account.id AND l.deadline>clock_timestamp())
  AND (approval.guardian_declaration_id IS NULL OR (med_tracker.sharing_declaration_eligible(approval.guardian_declaration_id,p_subject) AND EXISTS(
   SELECT 1 FROM public.guardian_declarations d
   JOIN public.carer_relationships c ON c.guardian_declaration_id=d.id AND c.active AND c.carer_id=d.guardian_person_id AND c.patient_id=d.subject_person_id
   JOIN public.accounts approver ON approver.id=approval.approved_by_account_id AND approver.person_id=d.guardian_person_id
   JOIN public.household_memberships gm ON gm.person_id=d.guardian_person_id AND gm.household_id=c.household_id
   JOIN public.household_memberships sm ON sm.person_id=d.subject_person_id AND sm.household_id=c.household_id
   WHERE d.id=approval.guardian_declaration_id AND d.guardian_person_id=approval.approved_by_person_id
    AND d.subject_person_id=p_subject AND d.withdrawn_at IS NULL AND d.superseded_at IS NULL
    AND approver.status=2 AND approver.active
    AND gm.status='active' AND gm.revoked_at IS NULL AND sm.status='active' AND sm.revoked_at IS NULL
  )))) INTO permitted;
 PERFORM set_config('med_tracker.sharing_subject_id',COALESCE(previous_subject,''),true);
 RETURN permitted;
END $body$;
ALTER FUNCTION med_tracker.sharing_current_member(bigint) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.sharing_current_member(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION med_tracker.sharing_current_member(bigint) TO med_tracker_app;

ALTER TABLE public.shared_schedule_states OWNER TO med_tracker_owner;
ALTER TABLE public.shared_schedule_states ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.shared_schedule_states FORCE ROW LEVEL SECURITY;
REVOKE ALL ON public.shared_schedule_states FROM PUBLIC,med_tracker_app;
GRANT SELECT,INSERT,UPDATE ON public.shared_schedule_states TO med_tracker_app;
CREATE FUNCTION med_tracker.sharing_origin_member(p_plan text,p_subject bigint) RETURNS boolean
LANGUAGE sql VOLATILE SET search_path=pg_catalog AS $body$
 SELECT EXISTS(SELECT 1 FROM public.schedules source JOIN public.household_memberships subject
  ON subject.person_id=source.person_id AND subject.household_id=source.household_id
  WHERE source.person_schedule_id=p_plan AND source.person_id=p_subject AND source.canonical_origin
   AND EXISTS(SELECT 1 FROM public.shared_schedule_executions link WHERE link.schedule_id=source.id
    AND link.approval_id IS NULL AND link.person_schedule_id=source.person_schedule_id
    AND link.person_id=source.person_id AND link.household_id=source.household_id AND link.portable_id=source.portable_id)
   AND NOT EXISTS(SELECT 1 FROM public.person_household_approvals prior WHERE prior.person_id=source.person_id AND prior.household_id=source.household_id)
   AND source.household_id=med_tracker.current_household_id() AND source.execution_ended_at IS NULL
   AND subject.status='active' AND subject.revoked_at IS NULL
   AND med_tracker.sharing_household_actor(source.household_id,false))
$body$;
ALTER FUNCTION med_tracker.sharing_origin_member(text,bigint) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.sharing_origin_member(text,bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION med_tracker.sharing_origin_member(text,bigint) TO med_tracker_app;
CREATE POLICY shared_schedule_read ON public.shared_schedule_states FOR SELECT
USING (med_tracker.sharing_clinical_authority(person_id) OR med_tracker.sharing_current_member(person_id)
 OR med_tracker.sharing_origin_member(id,person_id));
CREATE POLICY shared_schedule_insert ON public.shared_schedule_states FOR INSERT
WITH CHECK (med_tracker.sharing_clinical_authority(person_id));
CREATE POLICY shared_schedule_update ON public.shared_schedule_states FOR UPDATE
USING (med_tracker.sharing_clinical_authority(person_id) OR med_tracker.sharing_current_member(person_id) OR med_tracker.sharing_origin_member(id,person_id))
WITH CHECK (med_tracker.sharing_clinical_authority(person_id) OR med_tracker.sharing_current_member(person_id) OR med_tracker.sharing_origin_member(id,person_id));
CREATE FUNCTION med_tracker.shared_schedule_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $body$
BEGIN
 IF TG_OP='INSERT' OR (to_jsonb(NEW)-'latest_taken_at') IS DISTINCT FROM (to_jsonb(OLD)-'latest_taken_at') THEN
  IF NOT med_tracker.sharing_clinical_authority(NEW.person_id) THEN
   RAISE EXCEPTION 'Clinical authority required' USING ERRCODE='42501';
  END IF;
 END IF;
 IF TG_OP='UPDATE' AND (NEW.id<>OLD.id OR NEW.person_id<>OLD.person_id
  OR (OLD.latest_taken_at IS NOT NULL AND (NEW.latest_taken_at IS NULL OR NEW.latest_taken_at<OLD.latest_taken_at))) THEN
  RAISE EXCEPTION 'Shared identity and safety state are immutable' USING ERRCODE='42501';
 END IF;
 IF NEW.pause_instance_id IS NOT NULL AND (NEW.active OR NEW.pause_started_at IS NULL) THEN
  RAISE EXCEPTION 'Pause instance requires a current pause' USING ERRCODE='42501';
 END IF;
 IF TG_OP='UPDATE' THEN
  IF OLD.pause_instance_id IS NOT NULL AND NEW.pause_instance_id IS NULL THEN
   IF NOT NEW.active OR NEW.last_closed_pause_instance_id IS DISTINCT FROM OLD.pause_instance_id THEN
    RAISE EXCEPTION 'Pause closure requires the actual prior instance' USING ERRCODE='42501';
   END IF;
  ELSIF NEW.last_closed_pause_instance_id IS DISTINCT FROM OLD.last_closed_pause_instance_id THEN
   RAISE EXCEPTION 'Pause closure requires a resume' USING ERRCODE='42501';
  END IF;
 END IF;
 IF TG_OP='UPDATE' AND NEW.latest_taken_at IS DISTINCT FROM OLD.latest_taken_at
  AND NOT EXISTS(SELECT 1 FROM public.shared_schedule_administrations entry
   WHERE entry.person_schedule_id=NEW.id AND entry.household_id=med_tracker.current_household_id() AND entry.taken_at=NEW.latest_taken_at) THEN
  RAISE EXCEPTION 'Safety state requires an actual administration' USING ERRCODE='42501';
 END IF;
 RETURN NEW;
END $body$;
ALTER FUNCTION med_tracker.shared_schedule_guard() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.shared_schedule_guard() FROM PUBLIC,med_tracker_app;
CREATE TRIGGER shared_schedule_guard BEFORE INSERT OR UPDATE ON public.shared_schedule_states
FOR EACH ROW EXECUTE FUNCTION med_tracker.shared_schedule_guard();

ALTER TABLE public.shared_schedule_pause_closures OWNER TO med_tracker_owner;
ALTER TABLE public.shared_schedule_pause_closures ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.shared_schedule_pause_closures FORCE ROW LEVEL SECURITY;
REVOKE ALL ON public.shared_schedule_pause_closures FROM PUBLIC,med_tracker_app;
GRANT SELECT,INSERT ON public.shared_schedule_pause_closures TO med_tracker_app;
CREATE POLICY shared_pause_closure_owned_interval ON public.shared_schedule_pause_closures FOR SELECT TO med_tracker_app
USING (EXISTS(SELECT 1 FROM public.medication_pause_periods period
 JOIN public.schedules source ON source.id=period.schedule_id AND source.household_id=period.household_id
 JOIN public.household_memberships subject ON subject.person_id=source.person_id AND subject.household_id=source.household_id
 WHERE period.portable_id=shared_schedule_pause_closures.pause_instance_id AND source.person_schedule_id=shared_schedule_pause_closures.person_schedule_id
  AND period.canonical_person_schedule_id=shared_schedule_pause_closures.person_schedule_id
  AND period.canonical_pause_revision<shared_schedule_pause_closures.clinical_revision
  AND source.household_id=med_tracker.current_household_id() AND source.execution_ended_at IS NULL
  AND subject.status='active' AND subject.revoked_at IS NULL
  AND ((source.canonical_origin AND EXISTS(SELECT 1 FROM public.shared_schedule_executions link WHERE link.schedule_id=source.id
     AND link.approval_id IS NULL AND link.person_schedule_id=source.person_schedule_id
     AND link.person_id=source.person_id AND link.household_id=source.household_id AND link.portable_id=source.portable_id)
    AND NOT EXISTS(SELECT 1 FROM public.person_household_approvals prior WHERE prior.person_id=source.person_id AND prior.household_id=source.household_id))
   OR EXISTS(SELECT 1 FROM public.shared_schedule_executions link JOIN public.person_household_approvals approval ON approval.id=link.approval_id
    WHERE link.schedule_id=source.id AND link.person_schedule_id=source.person_schedule_id AND link.person_id=source.person_id
     AND link.household_id=source.household_id AND approval.household_id=source.household_id AND approval.person_id=source.person_id
     AND approval.ended_at IS NULL AND med_tracker.sharing_current_member(source.person_id)))
  AND med_tracker.sharing_household_actor(source.household_id,false)));
CREATE POLICY shared_pause_closure_actual_resume ON public.shared_schedule_pause_closures FOR INSERT TO med_tracker_app
WITH CHECK (actor_account_id=med_tracker.current_account_id()
 AND EXISTS(SELECT 1 FROM public.accounts actor JOIN public.shared_schedule_states state ON state.id=person_schedule_id
  WHERE actor.id=actor_account_id AND actor.person_id=actor_person_id
   AND med_tracker.sharing_clinical_authority(state.person_id)
   AND state.active AND state.pause_instance_id IS NULL AND state.last_closed_pause_instance_id=shared_schedule_pause_closures.pause_instance_id
   AND state.clinical_revision=shared_schedule_pause_closures.clinical_revision AND state.updated_at=ended_at
   AND ((actor.person_id=state.person_id AND guardian_declaration_id IS NULL)
    OR (actor.person_id<>state.person_id AND guardian_declaration_id=med_tracker.sharing_guardian(state.person_id)))
   AND (actor_membership_id IS NULL OR EXISTS(SELECT 1 FROM public.household_memberships member
    WHERE member.id=actor_membership_id AND member.person_id=actor.person_id AND member.household_id=med_tracker.current_household_id()
     AND member.status='active' AND member.revoked_at IS NULL))));
CREATE FUNCTION med_tracker.shared_pause_projection_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $body$
BEGIN
 IF TG_OP='INSERT' THEN
  IF NEW.canonical_resume_id IS NOT NULL OR NEW.canonical_person_schedule_id IS NOT NULL OR NEW.canonical_pause_revision IS NOT NULL THEN
   RAISE EXCEPTION 'Existing owned interval proof required' USING ERRCODE='42501';
  END IF;
  IF EXISTS(SELECT 1 FROM public.schedules source WHERE source.id=NEW.schedule_id AND source.household_id=NEW.household_id
   AND source.person_schedule_id IS NOT NULL AND NOT med_tracker.sharing_clinical_authority(source.person_id)) THEN
   RAISE EXCEPTION 'Clinical authority required' USING ERRCODE='42501';
  END IF;
  RETURN NEW;
 END IF;
 IF OLD.canonical_person_schedule_id IS NOT NULL THEN
  IF (to_jsonb(NEW)-ARRAY['canonical_resume_id','ended_at','resumed_by_membership_id','updated_at']) IS DISTINCT FROM
   (to_jsonb(OLD)-ARRAY['canonical_resume_id','ended_at','resumed_by_membership_id','updated_at'])
   OR EXISTS(SELECT 1 FROM public.schedules source WHERE source.id=OLD.schedule_id AND source.execution_ended_at IS NOT NULL) THEN
   RAISE EXCEPTION 'Owned pause identity is immutable' USING ERRCODE='42501';
  END IF;
 ELSIF NEW.canonical_person_schedule_id IS NOT NULL OR NEW.canonical_pause_revision IS NOT NULL THEN
  IF OLD.ended_at IS NOT NULL OR (to_jsonb(NEW)-ARRAY['canonical_person_schedule_id','canonical_pause_revision']) IS DISTINCT FROM
   (to_jsonb(OLD)-ARRAY['canonical_person_schedule_id','canonical_pause_revision'])
   OR NOT EXISTS(SELECT 1 FROM public.schedules source JOIN public.shared_schedule_states state ON state.id=source.person_schedule_id
    WHERE source.id=OLD.schedule_id AND source.household_id=OLD.household_id AND source.execution_ended_at IS NULL
     AND state.id=NEW.canonical_person_schedule_id AND state.clinical_revision=NEW.canonical_pause_revision
     AND NOT state.active AND state.pause_instance_id=OLD.portable_id AND state.pause_started_at=OLD.started_at
     AND med_tracker.sharing_clinical_authority(source.person_id)) THEN
   RAISE EXCEPTION 'Current clinical pause association required' USING ERRCODE='42501';
  END IF;
 END IF;
 IF OLD.canonical_resume_id IS NOT NULL AND (NEW.canonical_resume_id,NEW.ended_at,NEW.resumed_by_membership_id)
  IS DISTINCT FROM (OLD.canonical_resume_id,OLD.ended_at,OLD.resumed_by_membership_id) THEN
  RAISE EXCEPTION 'Canonical pause closure is immutable' USING ERRCODE='42501';
 END IF;
 IF NEW.canonical_resume_id IS NOT NULL AND OLD.canonical_resume_id IS NULL THEN
  IF (to_jsonb(NEW)-ARRAY['canonical_resume_id','ended_at','resumed_by_membership_id','updated_at']) IS DISTINCT FROM
   (to_jsonb(OLD)-ARRAY['canonical_resume_id','ended_at','resumed_by_membership_id','updated_at'])
   OR OLD.ended_at IS NOT NULL OR NOT EXISTS(SELECT 1 FROM public.shared_schedule_pause_closures closure
    WHERE closure.pause_instance_id=NEW.canonical_resume_id AND closure.pause_instance_id=OLD.portable_id
     AND closure.ended_at=NEW.ended_at AND NEW.resumed_by_membership_id IS NULL) THEN
   RAISE EXCEPTION 'Owned pause closure proof required' USING ERRCODE='42501';
  END IF;
 END IF;
 IF OLD.canonical_person_schedule_id IS NOT NULL AND NEW.canonical_resume_id IS NULL
  AND (NEW.ended_at,NEW.resumed_by_membership_id) IS DISTINCT FROM (OLD.ended_at,OLD.resumed_by_membership_id)
  AND NOT EXISTS(SELECT 1 FROM public.schedules source WHERE source.id=OLD.schedule_id AND med_tracker.sharing_clinical_authority(source.person_id)) THEN
  RAISE EXCEPTION 'Clinical authority required' USING ERRCODE='42501';
 END IF;
 RETURN NEW;
END $body$;
ALTER FUNCTION med_tracker.shared_pause_projection_guard() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.shared_pause_projection_guard() FROM PUBLIC,med_tracker_app;
CREATE TRIGGER shared_pause_projection_guard BEFORE INSERT OR UPDATE ON public.medication_pause_periods
FOR EACH ROW EXECUTE FUNCTION med_tracker.shared_pause_projection_guard();

CREATE FUNCTION med_tracker.sharing_attachment_authority(p_subject bigint,p_household bigint) RETURNS boolean
LANGUAGE sql VOLATILE SET search_path=pg_catalog AS $body$
 SELECT EXISTS(SELECT 1 FROM public.person_household_approvals approval
 JOIN public.accounts actor ON actor.id=approval.approved_by_account_id AND actor.id=med_tracker.current_account_id()
 JOIN public.household_invitations invitation ON invitation.id=approval.invitation_id
 WHERE approval.id::text=current_setting('med_tracker.sharing_attachment_approval_id',true)
  AND approval.person_id=p_subject AND approval.household_id=p_household AND approval.ended_at IS NULL
  AND approval.approved_by_person_id=actor.person_id AND actor.active AND actor.status=2
  AND med_tracker.sharing_clinical_authority(p_subject)
  AND ((actor.person_id=p_subject AND approval.guardian_declaration_id IS NULL)
   OR (actor.person_id<>p_subject AND approval.guardian_declaration_id=med_tracker.sharing_guardian(p_subject)))
  AND invitation.household_id=p_household AND invitation.accepted_at IS NOT NULL AND invitation.revoked_at IS NULL
  AND invitation.expires_at>timezone('UTC',clock_timestamp())
  AND invitation.token_digest=med_tracker.current_invitation_token_digest()
  AND lower(invitation.email)=lower(actor.email)
  AND invitation.dependent_sharing=(actor.person_id<>p_subject))
$body$;
ALTER FUNCTION med_tracker.sharing_attachment_authority(bigint,bigint) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.sharing_attachment_authority(bigint,bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION med_tracker.sharing_attachment_authority(bigint,bigint) TO med_tracker_app;

ALTER TABLE public.shared_schedule_executions OWNER TO med_tracker_owner;
ALTER TABLE public.shared_schedule_executions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.shared_schedule_executions FORCE ROW LEVEL SECURITY;
REVOKE ALL ON public.shared_schedule_executions FROM PUBLIC,med_tracker_app;
GRANT SELECT,INSERT ON public.shared_schedule_executions TO med_tracker_app;
GRANT UPDATE(approval_id) ON public.shared_schedule_executions TO med_tracker_app;
CREATE POLICY shared_execution_identifiers ON public.shared_schedule_executions FOR SELECT TO med_tracker_app
USING (med_tracker.sharing_household_actor(household_id,false)
 OR (approval_id::text=current_setting('med_tracker.sharing_withdrawal_approval_id',true)
  AND med_tracker.sharing_clinical_authority(person_id))
 OR med_tracker.sharing_attachment_authority(person_id,household_id));
CREATE POLICY shared_execution_identifier_insert ON public.shared_schedule_executions FOR INSERT TO med_tracker_app
WITH CHECK (household_id=med_tracker.current_household_id()
 AND EXISTS(SELECT 1 FROM public.schedules source
  WHERE source.id=schedule_id AND source.portable_id=shared_schedule_executions.portable_id
   AND source.household_id=shared_schedule_executions.household_id AND source.person_id=shared_schedule_executions.person_id
   AND source.person_schedule_id=shared_schedule_executions.person_schedule_id
   AND ((approval_id IS NULL AND source.canonical_origin AND source.sharing_approval_id IS NULL
    AND med_tracker.sharing_clinical_authority(source.person_id)
    AND NOT EXISTS(SELECT 1 FROM public.person_household_approvals prior WHERE prior.person_id=source.person_id AND prior.household_id=source.household_id))
   OR (source.execution_ended_at IS NULL AND med_tracker.sharing_current_member(source.person_id)
   AND (source.sharing_approval_id=approval_id OR (source.sharing_approval_id IS NULL AND source.canonical_origin
    AND med_tracker.sharing_clinical_authority(source.person_id)))
   AND EXISTS(SELECT 1 FROM public.person_household_approvals approval
    WHERE approval.id=approval_id AND approval.person_id=source.person_id
     AND approval.household_id=source.household_id AND approval.ended_at IS NULL)))));
CREATE POLICY shared_execution_origin_attachment ON public.shared_schedule_executions FOR UPDATE TO med_tracker_app
USING (approval_id IS NULL AND med_tracker.sharing_attachment_authority(person_id,household_id))
WITH CHECK (approval_id::text=current_setting('med_tracker.sharing_attachment_approval_id',true)
 AND med_tracker.sharing_attachment_authority(person_id,household_id));

-- EntityTrait update_many/QuerySelect and SeaQuery Query/Expr/Func express all
-- attachment DML and locks, but cannot define a privilege-bound validation-only
-- trigger over the foreign source's exact identity/end fact. No row is returned,
-- no trigger DML occurs, and only this matched source receives owner visibility.
CREATE POLICY sharing_origin_attachment_source ON public.schedules FOR SELECT TO med_tracker_owner
USING (id::text=current_setting('med_tracker.sharing_attachment_source_id',true));
CREATE POLICY sharing_origin_attachment_source_lock ON public.schedules FOR UPDATE TO med_tracker_owner
USING (id::text=current_setting('med_tracker.sharing_attachment_source_id',true)) WITH CHECK (false);
CREATE FUNCTION med_tracker.shared_origin_attachment_guard() RETURNS trigger
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=pg_catalog AS $body$
DECLARE previous_source text; actual record;
BEGIN
 IF OLD.approval_id IS NOT NULL OR NEW.approval_id IS NULL
  OR (to_jsonb(NEW)-'approval_id') IS DISTINCT FROM (to_jsonb(OLD)-'approval_id')
  OR NEW.approval_id::text IS DISTINCT FROM current_setting('med_tracker.sharing_attachment_approval_id',true)
  OR NOT med_tracker.sharing_attachment_authority(NEW.person_id,NEW.household_id) THEN
  RAISE EXCEPTION 'Exact current origin consent required' USING ERRCODE='42501';
 END IF;
 previous_source:=current_setting('med_tracker.sharing_attachment_source_id',true);
 PERFORM set_config('med_tracker.sharing_attachment_source_id',NEW.schedule_id::text,true);
 SELECT source.id,source.portable_id,source.person_id,source.household_id,source.person_schedule_id,
  source.canonical_origin,source.execution_ended_at INTO actual
 FROM public.schedules source WHERE source.id=NEW.schedule_id FOR SHARE;
 PERFORM set_config('med_tracker.sharing_attachment_source_id',COALESCE(previous_source,''),true);
 IF actual.id IS NULL OR NOT actual.canonical_origin OR actual.portable_id<>NEW.portable_id
  OR actual.person_id<>NEW.person_id OR actual.household_id<>NEW.household_id
  OR actual.person_schedule_id IS DISTINCT FROM NEW.person_schedule_id THEN
  RAISE EXCEPTION 'Exact current origin consent required' USING ERRCODE='42501';
 END IF;
 IF actual.execution_ended_at IS NOT NULL THEN RETURN NULL; END IF;
 IF NOT med_tracker.sharing_attachment_authority(NEW.person_id,NEW.household_id) THEN
  RAISE EXCEPTION 'Exact current origin consent required' USING ERRCODE='42501';
 END IF;
 RETURN NEW;
END $body$;
ALTER FUNCTION med_tracker.shared_origin_attachment_guard() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.shared_origin_attachment_guard() FROM PUBLIC,med_tracker_app;
CREATE TRIGGER shared_origin_attachment_guard BEFORE UPDATE ON public.shared_schedule_executions
FOR EACH ROW EXECUTE FUNCTION med_tracker.shared_origin_attachment_guard();

DROP INDEX public.schedules_person_execution;
CREATE UNIQUE INDEX schedules_person_execution ON public.schedules(person_schedule_id,household_id)
WHERE execution_ended_at IS NULL;
ALTER TABLE public.schedules ADD CONSTRAINT shared_schedule_no_private_notes CHECK(sharing_approval_id IS NULL OR notes IS NULL);
CREATE FUNCTION med_tracker.sharing_origin_execution(p_source bigint,p_plan text,p_subject bigint,p_household bigint,p_origin boolean) RETURNS boolean
LANGUAGE sql VOLATILE SET search_path=pg_catalog AS $body$
 SELECT p_origin AND p_household=med_tracker.current_household_id()
 AND EXISTS(SELECT 1 FROM public.household_memberships subject WHERE subject.person_id=p_subject
  AND subject.household_id=p_household AND subject.status='active' AND subject.revoked_at IS NULL)
 AND ((EXISTS(SELECT 1 FROM public.shared_schedule_executions link WHERE link.schedule_id=p_source
   AND link.person_schedule_id=p_plan AND link.person_id=p_subject AND link.household_id=p_household AND link.approval_id IS NULL)
  AND NOT EXISTS(SELECT 1 FROM public.person_household_approvals prior WHERE prior.person_id=p_subject AND prior.household_id=p_household))
 OR (NOT EXISTS(SELECT 1 FROM public.shared_schedule_executions link WHERE link.schedule_id=p_source)
  AND med_tracker.sharing_clinical_authority(p_subject)
  AND (med_tracker.sharing_current_member(p_subject)
   OR NOT EXISTS(SELECT 1 FROM public.person_household_approvals prior WHERE prior.person_id=p_subject AND prior.household_id=p_household))))
$body$;
ALTER FUNCTION med_tracker.sharing_origin_execution(bigint,text,bigint,bigint,boolean) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.sharing_origin_execution(bigint,text,bigint,bigint,boolean) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION med_tracker.sharing_origin_execution(bigint,text,bigint,bigint,boolean) TO med_tracker_app;
CREATE POLICY shared_schedule_current_execution ON public.schedules AS RESTRICTIVE
FOR ALL TO med_tracker_app
USING ((sharing_approval_id IS NULL AND person_schedule_id IS NULL)
 OR med_tracker.sharing_origin_execution(id,person_schedule_id,person_id,household_id,canonical_origin) OR execution_ended_at IS NOT NULL
 OR (med_tracker.sharing_current_member(person_id) AND EXISTS(SELECT 1 FROM public.person_household_approvals approval WHERE approval.ended_at IS NULL
  AND (approval.id=sharing_approval_id OR EXISTS(SELECT 1 FROM public.shared_schedule_executions link WHERE link.schedule_id=schedules.id AND link.approval_id=approval.id))))
 OR (med_tracker.sharing_clinical_authority(person_id) AND (sharing_approval_id::text=current_setting('med_tracker.sharing_withdrawal_approval_id',true)
  OR EXISTS(SELECT 1 FROM public.shared_schedule_executions link WHERE link.schedule_id=schedules.id AND link.approval_id::text=current_setting('med_tracker.sharing_withdrawal_approval_id',true)))))
WITH CHECK ((sharing_approval_id IS NULL AND person_schedule_id IS NULL)
 OR med_tracker.sharing_origin_execution(id,person_schedule_id,person_id,household_id,canonical_origin) OR execution_ended_at IS NOT NULL
 OR (med_tracker.sharing_current_member(person_id) AND EXISTS(SELECT 1 FROM public.person_household_approvals approval WHERE approval.ended_at IS NULL
  AND (approval.id=sharing_approval_id OR EXISTS(SELECT 1 FROM public.shared_schedule_executions link WHERE link.schedule_id=schedules.id AND link.approval_id=approval.id)))));
CREATE POLICY shared_schedule_withdrawal ON public.schedules FOR UPDATE TO med_tracker_app
USING (execution_ended_at IS NULL AND med_tracker.sharing_clinical_authority(person_id)
 AND (sharing_approval_id::text=current_setting('med_tracker.sharing_withdrawal_approval_id',true)
  OR EXISTS(SELECT 1 FROM public.shared_schedule_executions link WHERE link.schedule_id=schedules.id AND link.approval_id::text=current_setting('med_tracker.sharing_withdrawal_approval_id',true))))
WITH CHECK (med_tracker.sharing_clinical_authority(person_id)
 AND (sharing_approval_id::text=current_setting('med_tracker.sharing_withdrawal_approval_id',true)
  OR EXISTS(SELECT 1 FROM public.shared_schedule_executions link WHERE link.schedule_id=schedules.id AND link.approval_id::text=current_setting('med_tracker.sharing_withdrawal_approval_id',true))));
CREATE POLICY sharing_withdrawal_exact_update ON public.schedules AS RESTRICTIVE FOR UPDATE TO med_tracker_app
USING (NULLIF(current_setting('med_tracker.sharing_withdrawal_approval_id',true),'') IS NULL
 OR (execution_ended_at IS NULL AND med_tracker.sharing_clinical_authority(person_id)
  AND (sharing_approval_id::text=current_setting('med_tracker.sharing_withdrawal_approval_id',true)
   OR EXISTS(SELECT 1 FROM public.shared_schedule_executions link WHERE link.schedule_id=schedules.id AND link.approval_id::text=current_setting('med_tracker.sharing_withdrawal_approval_id',true)))))
WITH CHECK (NULLIF(current_setting('med_tracker.sharing_withdrawal_approval_id',true),'') IS NULL
 OR (med_tracker.sharing_clinical_authority(person_id)
  AND (sharing_approval_id::text=current_setting('med_tracker.sharing_withdrawal_approval_id',true)
   OR EXISTS(SELECT 1 FROM public.shared_schedule_executions link WHERE link.schedule_id=schedules.id AND link.approval_id::text=current_setting('med_tracker.sharing_withdrawal_approval_id',true)))));
CREATE FUNCTION med_tracker.shared_execution_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $body$
BEGIN
 IF NEW.canonical_origin AND (TG_OP='INSERT' OR NOT OLD.canonical_origin) THEN
  IF (current_user='med_tracker_app' AND NOT med_tracker.sharing_clinical_authority(NEW.person_id))
   OR NOT EXISTS(SELECT 1 FROM public.person_schedules canonical WHERE canonical.id=NEW.person_schedule_id
    AND canonical.person_id=NEW.person_id AND canonical.origin_household_id=NEW.household_id AND canonical.origin_schedule_id=NEW.id) THEN
   RAISE EXCEPTION 'Exact clinical origin association required' USING ERRCODE='42501';
  END IF;
 END IF;
 IF TG_OP='UPDATE' THEN
  IF NEW.id<>OLD.id OR NEW.portable_id<>OLD.portable_id OR NEW.person_id<>OLD.person_id OR NEW.household_id<>OLD.household_id
   OR (OLD.person_schedule_id IS NOT NULL AND NEW.person_schedule_id IS DISTINCT FROM OLD.person_schedule_id)
   OR NEW.sharing_approval_id IS DISTINCT FROM OLD.sharing_approval_id OR (OLD.canonical_origin AND NOT NEW.canonical_origin) THEN
   RAISE EXCEPTION 'Execution identity is immutable' USING ERRCODE='42501';
  END IF;
  IF current_user='med_tracker_app' AND OLD.person_schedule_id IS NULL AND NEW.person_schedule_id IS NOT NULL
   AND NOT med_tracker.sharing_clinical_authority(NEW.person_id) THEN
   RAISE EXCEPTION 'Clinical association required' USING ERRCODE='42501';
  END IF;
  IF OLD.person_schedule_id IS NOT NULL AND OLD.execution_ended_at IS NOT NULL AND NEW IS DISTINCT FROM OLD THEN
   RAISE EXCEPTION 'Ended execution is immutable' USING ERRCODE='42501';
  END IF;
  IF OLD.execution_ended_at IS NULL AND NEW.execution_ended_at IS NOT NULL THEN
   IF NEW.active OR (to_jsonb(NEW)-ARRAY['execution_ended_at','active','updated_at']) IS DISTINCT FROM (to_jsonb(OLD)-ARRAY['execution_ended_at','active','updated_at'])
    OR NOT (med_tracker.sharing_clinical_authority(OLD.person_id) OR med_tracker.sharing_household_actor(OLD.household_id,true)) THEN
    RAISE EXCEPTION 'Only terminal closure is permitted' USING ERRCODE='42501';
   END IF;
   RETURN NEW;
  END IF;
 END IF;
 IF NEW.person_schedule_id IS NOT NULL AND NOT med_tracker.sharing_clinical_authority(NEW.person_id) THEN
  IF NOT EXISTS(SELECT 1 FROM public.shared_schedule_states state WHERE state.id=NEW.person_schedule_id
   AND state.person_id=NEW.person_id
   AND (NEW.active,NEW.retired_at,NEW.dose_amount,NEW.dose_unit,NEW.dose_cycle,NEW.frequency,NEW.start_date,NEW.end_date,
    NEW.max_daily_doses,NEW.min_hours_between_doses,NEW.schedule_type,NEW.schedule_config,NEW.origin_time_zone,NEW.clinical_revision)
    IS NOT DISTINCT FROM
    (state.active,state.retired_at,state.dose_amount,state.dose_unit,state.dose_cycle,state.frequency,state.start_date,state.end_date,
     state.max_daily_doses,state.min_hours_between_doses,state.schedule_type,state.schedule_config,state.origin_time_zone,state.clinical_revision))
   OR (TG_OP='INSERT' AND (NEW.sharing_approval_id IS NULL OR NEW.notes IS NOT NULL OR NOT med_tracker.sharing_household_actor(NEW.household_id,true))) THEN
   RAISE EXCEPTION 'Clinical authority required' USING ERRCODE='42501';
  END IF;
  IF TG_OP='UPDATE' AND (NEW.notes IS DISTINCT FROM OLD.notes OR NEW.created_at<>OLD.created_at
   OR (NEW.medication_id<>OLD.medication_id AND NOT med_tracker.sharing_household_actor(NEW.household_id,true))) THEN
   RAISE EXCEPTION 'Clinical authority required' USING ERRCODE='42501';
  END IF;
 END IF;
 IF TG_OP='INSERT' THEN RETURN NEW; END IF;
 IF OLD.sharing_approval_id IS NOT NULL OR EXISTS(SELECT 1 FROM public.shared_schedule_executions link WHERE link.schedule_id=OLD.id) THEN
  IF OLD.execution_ended_at IS NOT NULL AND NEW IS DISTINCT FROM OLD THEN
   RAISE EXCEPTION 'Ended execution is immutable' USING ERRCODE='42501';
  END IF;
  IF OLD.sharing_approval_id::text=current_setting('med_tracker.sharing_withdrawal_approval_id',true)
   OR EXISTS(SELECT 1 FROM public.shared_schedule_executions link WHERE link.schedule_id=OLD.id AND link.approval_id::text=current_setting('med_tracker.sharing_withdrawal_approval_id',true)) THEN
   IF NOT med_tracker.sharing_clinical_authority(OLD.person_id) OR NEW.execution_ended_at IS NULL OR NEW.active
    OR (to_jsonb(NEW)-ARRAY['execution_ended_at','active','updated_at']) IS DISTINCT FROM (to_jsonb(OLD)-ARRAY['execution_ended_at','active','updated_at']) THEN
    RAISE EXCEPTION 'Only terminal closure is permitted' USING ERRCODE='42501';
   END IF;
  END IF;
 END IF;
 RETURN NEW;
END $body$;
ALTER FUNCTION med_tracker.shared_execution_guard() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.shared_execution_guard() FROM PUBLIC,med_tracker_app;
CREATE TRIGGER shared_execution_guard BEFORE INSERT OR UPDATE ON public.schedules FOR EACH ROW EXECUTE FUNCTION med_tracker.shared_execution_guard();
CREATE POLICY person_schedule_clinical_insert ON public.person_schedules AS RESTRICTIVE FOR INSERT TO med_tracker_app
WITH CHECK (med_tracker.sharing_clinical_authority(person_id));
CREATE POLICY person_schedule_clinical_update ON public.person_schedules AS RESTRICTIVE FOR UPDATE TO med_tracker_app
USING (med_tracker.sharing_clinical_authority(person_id)) WITH CHECK (med_tracker.sharing_clinical_authority(person_id));
-- SeaORM update_many with constant SET/WHERE and no RETURNING provides the
-- command DML without private canonical SELECT. SeaQuery cannot express policy
-- or validation-trigger DDL; this exact UUID/captured-declaration invoker scope
-- adds UPDATE only, never a reader or a privileged execution surface.
CREATE FUNCTION med_tracker.sharing_clinical_command(p_plan text,p_subject bigint) RETURNS boolean
LANGUAGE sql VOLATILE SET search_path=pg_catalog AS $body$
 SELECT COALESCE(p_plan=current_setting('med_tracker.sharing_clinical_plan_id',true)
 AND med_tracker.sharing_clinical_authority(p_subject)
 AND ((current_setting('med_tracker.sharing_clinical_declaration_id',true)='self'
  AND EXISTS(SELECT 1 FROM public.accounts actor WHERE actor.id=med_tracker.current_account_id() AND actor.person_id=p_subject AND actor.active AND actor.status=2))
 OR med_tracker.sharing_guardian(p_subject)::text=current_setting('med_tracker.sharing_clinical_declaration_id',true)),false)
$body$;
ALTER FUNCTION med_tracker.sharing_clinical_command(text,bigint) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.sharing_clinical_command(text,bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION med_tracker.sharing_clinical_command(text,bigint) TO med_tracker_app;
CREATE POLICY sharing_clinical_command ON public.person_schedules FOR UPDATE TO med_tracker_app
USING (med_tracker.sharing_clinical_command(id,person_id)) WITH CHECK (med_tracker.sharing_clinical_command(id,person_id));
CREATE POLICY sharing_clinical_command_scope ON public.person_schedules AS RESTRICTIVE FOR UPDATE TO med_tracker_app
USING (NULLIF(current_setting('med_tracker.sharing_clinical_plan_id',true),'') IS NULL OR med_tracker.sharing_clinical_command(id,person_id))
WITH CHECK (NULLIF(current_setting('med_tracker.sharing_clinical_plan_id',true),'') IS NULL OR med_tracker.sharing_clinical_command(id,person_id));
CREATE FUNCTION med_tracker.shared_canonical_identity_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $body$
DECLARE allowed text[];
BEGIN
 IF (NEW.id,NEW.person_id,NEW.origin_household_id,NEW.origin_schedule_id,NEW.created_at)
  IS DISTINCT FROM (OLD.id,OLD.person_id,OLD.origin_household_id,OLD.origin_schedule_id,OLD.created_at) THEN
  RAISE EXCEPTION 'Canonical origin identity is immutable' USING ERRCODE='42501';
 END IF;
 IF NULLIF(current_setting('med_tracker.sharing_clinical_plan_id',true),'') IS NOT NULL THEN
  IF NOT med_tracker.sharing_clinical_command(OLD.id,OLD.person_id) OR NEW.clinical_revision<>OLD.clinical_revision+1 THEN
   RAISE EXCEPTION 'Current captured clinical authority required' USING ERRCODE='42501';
  END IF;
  allowed:=ARRAY['clinical_revision','updated_at','active','retired_at','dose_amount','dose_unit','dose_cycle','frequency','start_date','end_date',
   'max_daily_doses','min_hours_between_doses','schedule_type','schedule_config','origin_time_zone','timezone_provenance','pause_started_at','pause_instance_id',
   'pause_reason','pause_note'];
  IF NULLIF(current_setting('med_tracker.sharing_clinical_stock_id',true),'') IS NOT NULL THEN
   IF NOT med_tracker.sharing_household_actor(med_tracker.current_household_id(),false)
    OR NOT EXISTS(SELECT 1 FROM public.household_memberships subject WHERE subject.person_id=OLD.person_id
     AND subject.household_id=med_tracker.current_household_id() AND subject.status='active' AND subject.revoked_at IS NULL)
    OR NOT EXISTS(SELECT 1 FROM public.medications stock WHERE stock.id::text=current_setting('med_tracker.sharing_clinical_stock_id',true)
     AND stock.household_id=med_tracker.current_household_id() AND stock.dmd_system='https://dmd.nhs.uk'
     AND stock.dmd_concept_class IN ('VMP','AMP','VMPP','AMPP') AND length(trim(stock.dmd_code))>0
     AND (NEW.medicine_name,NEW.dmd_system,NEW.dmd_code,NEW.dmd_concept_class,NEW.stock_unit)
      IS NOT DISTINCT FROM (stock.name,stock.dmd_system,stock.dmd_code,stock.dmd_concept_class,stock.dose_unit)
     AND NEW.dose_unit=stock.dose_unit AND NEW.dose_amount>0
     AND ((NEW.stock_default_known AND NEW.stock_dose_default::double precision IS NOT DISTINCT FROM stock.dose_amount)
      OR (NOT NEW.stock_default_known AND NEW.stock_dose_default IS NULL))) THEN
    RAISE EXCEPTION 'Current local prescription stock required' USING ERRCODE='42501';
   END IF;
   allowed:=allowed||ARRAY['medicine_name','dmd_system','dmd_code','dmd_concept_class','stock_unit','stock_dose_default','stock_default_known'];
  END IF;
  IF (to_jsonb(NEW)-allowed) IS DISTINCT FROM (to_jsonb(OLD)-allowed)
   OR ((NEW.pause_reason,NEW.pause_note) IS DISTINCT FROM (OLD.pause_reason,OLD.pause_note)
    AND NOT (NEW.active<>OLD.active AND NEW.pause_reason IS NULL AND NEW.pause_note IS NULL))
   OR (NEW.timezone_provenance IS DISTINCT FROM OLD.timezone_provenance
    AND NOT (NEW.origin_time_zone<>OLD.origin_time_zone AND NEW.timezone_provenance='authorised_correction')) THEN
   RAISE EXCEPTION 'Safe clinical fields only' USING ERRCODE='42501';
  END IF;
 END IF;
 RETURN NEW;
END $body$;
ALTER FUNCTION med_tracker.shared_canonical_identity_guard() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.shared_canonical_identity_guard() FROM PUBLIC,med_tracker_app;
CREATE TRIGGER shared_canonical_identity_guard BEFORE UPDATE ON public.person_schedules
FOR EACH ROW EXECUTE FUNCTION med_tracker.shared_canonical_identity_guard();

-- RLS policy/role DDL is not expressible by SeaORM EntityTrait or SeaQuery's
-- query builders. Keep existing append and verifier/owner capabilities; runtime
-- reads follow current household or target ownership, never historical authorship.
ALTER TABLE public.versions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.versions FORCE ROW LEVEL SECURITY;
CREATE POLICY sharing_versions_append ON public.versions FOR INSERT TO med_tracker_app,med_tracker_owner WITH CHECK (true);
CREATE POLICY sharing_versions_verifier ON public.versions FOR SELECT TO med_tracker_owner,med_tracker_audit_verifier USING (true);
CREATE POLICY sharing_versions_current_reader ON public.versions FOR SELECT TO med_tracker_app
USING (EXISTS(SELECT 1 FROM public.accounts actor WHERE actor.id=med_tracker.current_account_id()
 AND actor.active AND actor.status=2
 AND NOT EXISTS(SELECT 1 FROM public.account_lockouts locked WHERE locked.account_id=actor.id AND locked.deadline>clock_timestamp())
 AND ((household_id=med_tracker.current_household_id() AND med_tracker.sharing_household_actor(household_id,false))
  OR (item_type IN ('Account','AuthenticationToken','AccountClinicalAccess') AND item_id=actor.id)
  OR (item_type='Person' AND item_id=actor.person_id)
  OR (item_type='PersonSchedule' AND EXISTS(SELECT 1 FROM public.person_schedules plan WHERE plan.origin_schedule_id=item_id AND plan.person_id=actor.person_id))
  OR (item_type='GuardianDeclaration' AND EXISTS(SELECT 1 FROM public.guardian_declarations declaration WHERE declaration.id=item_id AND declaration.guardian_person_id=actor.person_id))
  OR (item_type='PersonHouseholdApproval' AND EXISTS(SELECT 1 FROM public.person_household_approvals approval WHERE approval.id=item_id AND med_tracker.sharing_clinical_authority(approval.person_id)))
  OR (item_type='SharedPersonSchedule' AND med_tracker.sharing_clinical_authority(item_id)))));
CREATE POLICY sharing_withdrawal_tombstone ON public.api_tombstones FOR INSERT TO med_tracker_app
WITH CHECK (record_type='Schedule' AND action='delete' AND household_membership_id IS NULL
 AND account_id=med_tracker.current_account_id()
 AND (metadata-ARRAY['record_type','record_id','portable_id','person_portable_id','approval_id'])='{}'::jsonb
 AND metadata->>'record_type'='Schedule' AND metadata->>'portable_id'=record_portable_id
 AND EXISTS(SELECT 1 FROM public.shared_schedule_executions source
  JOIN public.person_household_approvals approval ON approval.id=source.approval_id
  JOIN public.people subject ON subject.id=source.person_id
  WHERE source.approval_id::text=current_setting('med_tracker.sharing_withdrawal_approval_id',true)
   AND source.household_id=api_tombstones.household_id AND source.portable_id=record_portable_id
   AND source.schedule_id::text=metadata->>'record_id' AND source.approval_id::text=metadata->>'approval_id'
   AND subject.portable_id=metadata->>'person_portable_id' AND approval.ended_at IS NOT NULL
   AND med_tracker.sharing_clinical_authority(source.person_id)));
CREATE POLICY sharing_guardian_subject ON public.people FOR SELECT TO med_tracker_app
USING (med_tracker.sharing_guardian(id) IS NOT NULL);

ALTER TABLE public.shared_schedule_administrations OWNER TO med_tracker_owner;
ALTER TABLE public.shared_schedule_administrations ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.shared_schedule_administrations FORCE ROW LEVEL SECURITY;
REVOKE ALL ON public.shared_schedule_administrations FROM PUBLIC,med_tracker_app;
GRANT SELECT,INSERT ON public.shared_schedule_administrations TO med_tracker_app;
GRANT UPDATE(occurrence_starts_on,occurrence_position) ON public.shared_schedule_administrations TO med_tracker_app;
GRANT USAGE ON SEQUENCE public.shared_schedule_administrations_id_seq TO med_tracker_app;
CREATE UNIQUE INDEX shared_administration_occurrence
ON public.shared_schedule_administrations(person_schedule_id,occurrence_starts_on,occurrence_position);
CREATE POLICY shared_administration_local ON public.shared_schedule_administrations TO med_tracker_app
USING (household_id=med_tracker.current_household_id())
WITH CHECK (household_id=med_tracker.current_household_id()
 AND EXISTS(SELECT 1 FROM public.medication_takes take JOIN public.schedules source ON source.id=take.schedule_id
  WHERE take.id=medication_take_id AND take.household_id=shared_schedule_administrations.household_id
   AND source.person_schedule_id=shared_schedule_administrations.person_schedule_id
   AND source.execution_ended_at IS NULL));
CREATE POLICY shared_administration_safety_owner ON public.shared_schedule_administrations FOR SELECT TO med_tracker_owner
USING (person_schedule_id=NULLIF(current_setting('med_tracker.sharing_schedule_id',true),''));

CREATE POLICY sharing_recording_house_owner ON public.households FOR SELECT TO med_tracker_owner
USING (id=med_tracker.current_household_id()
 AND NULLIF(current_setting('med_tracker.sharing_take_id',true),'') IS NOT NULL
 AND EXISTS(SELECT 1 FROM public.household_memberships member JOIN public.accounts actor ON actor.person_id=member.person_id
  WHERE member.id=med_tracker.current_membership_id() AND member.household_id=households.id
   AND actor.id=med_tracker.current_account_id() AND actor.active AND actor.status=2
   AND member.status='active' AND member.revoked_at IS NULL));
CREATE POLICY sharing_recording_take_owner ON public.medication_takes FOR SELECT TO med_tracker_owner
USING (household_id=med_tracker.current_household_id() AND id::text=current_setting('med_tracker.sharing_take_id',true));
CREATE POLICY sharing_recording_source_owner ON public.schedules FOR SELECT TO med_tracker_owner
USING (household_id=med_tracker.current_household_id() AND id::text=current_setting('med_tracker.sharing_source_id',true));
CREATE POLICY sharing_recording_link_owner ON public.shared_schedule_executions FOR SELECT TO med_tracker_owner
USING (household_id=med_tracker.current_household_id() AND schedule_id::text=current_setting('med_tracker.sharing_source_id',true));

-- SeaORM EntityTrait/QuerySelect and SeaQuery Query/Expr/Func express ordinary
-- queries, but not an owner/ACL-bound validation trigger over RLS-hidden safety
-- facts. This trigger has no runtime EXECUTE surface or caller-selected windows.
-- It returns NULL only for the fixed timing denial; the single plain SeaORM
-- ledger insert maps RecordNotInserted to a domain error and rolls back the take.
CREATE FUNCTION med_tracker.shared_administration_guard() RETURNS trigger
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
  IF NEW.household_id<>med_tracker.current_household_id()
   OR NOT med_tracker.sharing_household_actor(NEW.household_id,false) THEN
   RAISE EXCEPTION 'Original administration evidence unavailable' USING ERRCODE='42501';
  END IF;
  SELECT take.* INTO actual_take FROM public.medication_takes take
  WHERE take.id=NEW.medication_take_id AND take.household_id=NEW.household_id;
  IF NOT FOUND OR actual_take.schedule_id IS NULL OR actual_take.person_medication_id IS NOT NULL THEN
   RAISE EXCEPTION 'Original administration evidence unavailable' USING ERRCODE='42501';
  END IF;
  PERFORM set_config('med_tracker.sharing_source_id',actual_take.schedule_id::text,true);
  SELECT source.* INTO actual_source FROM public.schedules source WHERE source.id=actual_take.schedule_id
   AND source.household_id=NEW.household_id AND source.person_schedule_id IS NULL AND source.execution_ended_at IS NULL FOR SHARE;
  IF NOT FOUND THEN RAISE EXCEPTION 'Original administration evidence unavailable' USING ERRCODE='42501'; END IF;
  PERFORM set_config('med_tracker.sharing_subject_id',actual_source.person_id::text,true);
  IF NOT med_tracker.sharing_clinical_authority(actual_source.person_id)
   OR NEW.captured_by_account_id<>med_tracker.current_account_id()
   OR NEW.captured_by_membership_id<>med_tracker.current_membership_id()
   OR NOT EXISTS(SELECT 1 FROM public.accounts actor WHERE actor.id=NEW.captured_by_account_id AND actor.person_id=NEW.captured_by_person_id)
   OR (NEW.captured_by_person_id=actual_source.person_id AND NEW.guardian_declaration_id IS NOT NULL)
   OR (NEW.captured_by_person_id<>actual_source.person_id AND NEW.guardian_declaration_id IS DISTINCT FROM med_tracker.sharing_guardian(actual_source.person_id))
   OR NOT EXISTS(SELECT 1 FROM public.person_schedules plan WHERE plan.id=NEW.person_schedule_id
    AND plan.origin_schedule_id=actual_source.id AND plan.origin_household_id=actual_source.household_id AND plan.person_id=actual_source.person_id)
   OR (NEW.source_schedule_id,NEW.source_portable_id,NEW.person_id,NEW.household_id)
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
 IF NEW.household_id<>med_tracker.current_household_id()
  OR NOT med_tracker.sharing_household_actor(NEW.household_id,false)
  OR NEW.occurrence_starts_on IS NOT NULL OR NEW.occurrence_position IS NOT NULL THEN
  RAISE EXCEPTION 'Shared administration unavailable' USING ERRCODE='42501';
 END IF;
 SELECT take.* INTO actual_take
 FROM public.medication_takes take WHERE take.id=NEW.medication_take_id AND take.household_id=NEW.household_id;
 IF NOT FOUND OR actual_take.schedule_id IS NULL OR actual_take.taken_at IS NULL
  OR actual_take.taken_at IS DISTINCT FROM NEW.taken_at OR actual_take.dose_amount IS NULL OR actual_take.dose_amount<=0 THEN
  RAISE EXCEPTION 'Shared administration unavailable' USING ERRCODE='42501';
 END IF;
 PERFORM set_config('med_tracker.sharing_source_id',actual_take.schedule_id::text,true);
 SELECT fact.* INTO captured FROM public.shared_schedule_adoption_facts fact WHERE fact.medication_take_id=NEW.medication_take_id;
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
    actual_take.taken_from_medication_id,actual_take.taken_from_location_id,actual_take.created_at,actual_take.updated_at) THEN
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
CREATE TRIGGER shared_administration_guard BEFORE INSERT ON public.shared_schedule_administrations
FOR EACH ROW EXECUTE FUNCTION med_tracker.shared_administration_guard();

-- SeaORM EntityTrait/SeaQuery express the reservation and delivery DML, but
-- provide no trigger/function DDL builder. This invoker guard compares OLD/NEW
-- only: it grants no reader or privileged query and never writes another row.
-- Runtime cannot move the original content clock. The database owner retains
-- maintenance/fixture clock control; delivery retry/lease times are separate.
CREATE FUNCTION med_tracker.shared_reminder_epoch_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $body$
BEGIN
 IF TG_OP='DELETE' THEN
  IF current_user='med_tracker_app' AND OLD.event_type IN ('dose_due','missed_dose','low_stock') THEN
   RAISE EXCEPTION 'Reminder history is immutable' USING ERRCODE='42501';
  END IF;
  RETURN OLD;
 END IF;
 IF OLD.event_type IN ('dose_due','missed_dose','low_stock') OR NEW.event_type IN ('dose_due','missed_dose','low_stock') THEN
  IF current_user='med_tracker_app' AND
   (NEW.intended_at,NEW.expires_at) IS DISTINCT FROM (OLD.intended_at,OLD.expires_at) THEN
   RAISE EXCEPTION 'Reminder content deadline is immutable' USING ERRCODE='42501';
  END IF;
  IF (NEW.id,NEW.household_id,NEW.person_id,NEW.event_key,NEW.event_type,NEW.created_at,
   NEW.metadata->'account_id',NEW.metadata->'sharing_approval_id',NEW.metadata->'subject_membership_id',
   NEW.metadata->'subject_permissions_version',NEW.metadata->'sources',NEW.metadata->'base_event_key')
   IS DISTINCT FROM
   (OLD.id,OLD.household_id,OLD.person_id,OLD.event_key,OLD.event_type,OLD.created_at,
   OLD.metadata->'account_id',OLD.metadata->'sharing_approval_id',OLD.metadata->'subject_membership_id',
   OLD.metadata->'subject_permissions_version',OLD.metadata->'sources',OLD.metadata->'base_event_key') THEN
   RAISE EXCEPTION 'Reminder care proof is immutable' USING ERRCODE='42501';
  END IF;
 END IF;
 RETURN NEW;
END $body$;
ALTER FUNCTION med_tracker.shared_reminder_epoch_guard() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.shared_reminder_epoch_guard() FROM PUBLIC,med_tracker_app;
CREATE TRIGGER shared_reminder_epoch_guard BEFORE UPDATE OR DELETE ON public.notification_events
FOR EACH ROW EXECUTE FUNCTION med_tracker.shared_reminder_epoch_guard();
