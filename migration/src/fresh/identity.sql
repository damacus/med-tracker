ALTER TABLE public.accounts ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.accounts FORCE ROW LEVEL SECURITY;
ALTER TABLE public.households ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.households FORCE ROW LEVEL SECURITY;

CREATE POLICY identity_owner ON public.accounts TO med_tracker_owner USING (true) WITH CHECK (true);
CREATE POLICY identity_owner ON public.people TO med_tracker_owner USING (true) WITH CHECK (true);
CREATE POLICY identity_owner ON public.household_memberships TO med_tracker_owner USING (true) WITH CHECK (true);
CREATE POLICY identity_owner ON public.households TO med_tracker_owner USING (true) WITH CHECK (true);

CREATE FUNCTION med_tracker.current_person_id() RETURNS bigint
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$ SELECT person_id FROM public.accounts WHERE id = med_tracker.current_account_id() AND status IN (1, 2) AND disabled_at IS NULL $$;

CREATE FUNCTION med_tracker.has_household_access(target_household bigint) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$
  SELECT EXISTS (
    SELECT 1 FROM public.household_memberships membership
    JOIN public.accounts account ON account.person_id = membership.person_id
    JOIN public.households household ON household.id = membership.household_id
    WHERE membership.household_id = target_household
      AND account.id = med_tracker.current_account_id() AND account.status = 2 AND account.disabled_at IS NULL
      AND membership.status = 'active' AND membership.revoked_at IS NULL
      AND household.status = 'active' AND household.lifecycle_state = 'active'
      AND NOT EXISTS (SELECT 1 FROM public.account_lockouts lockout
        WHERE lockout.account_id = account.id AND lockout.deadline > CURRENT_TIMESTAMP)
  )
$$;

CREATE OR REPLACE FUNCTION med_tracker.current_household_id() RETURNS bigint
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$
  SELECT CASE WHEN med_tracker.has_household_access(NULLIF(current_setting('med_tracker.current_household_id', true), '')::bigint)
    THEN NULLIF(current_setting('med_tracker.current_household_id', true), '')::bigint END
$$;

CREATE POLICY versions_stock_removal_history ON public.versions FOR SELECT TO med_tracker_app
USING (item_type = 'MedicationStockRemoval' AND household_id = med_tracker.current_household_id()
  AND EXISTS (
    SELECT 1 FROM public.household_memberships member
    WHERE member.id = med_tracker.current_membership_id() AND member.household_id = public.versions.household_id
      AND member.person_id = med_tracker.current_person_id()
      AND member.status = 'active' AND member.revoked_at IS NULL
      AND member.role IN ('owner', 'administrator')
  ));

CREATE POLICY versions_current_actor_household ON public.versions FOR SELECT TO med_tracker_app
USING (household_id = med_tracker.current_household_id()
  AND item_type <> 'MedicationStockRemoval'
  AND actor_membership_id = med_tracker.current_membership_id()
  AND whodunnit = med_tracker.current_person_id()::text
  AND EXISTS (
    SELECT 1 FROM public.household_memberships member
    WHERE member.id = actor_membership_id AND member.household_id = public.versions.household_id
      AND member.person_id = med_tracker.current_person_id()
      AND member.status = 'active' AND member.revoked_at IS NULL
  )
  AND (item_type NOT IN ('Account', 'AuthenticationToken') OR item_id = med_tracker.current_account_id())
  AND (NOT (audit_context ? 'actor_account_id') OR audit_context->>'actor_account_id' = med_tracker.current_account_id()::text)
  AND (NOT (audit_context ? 'actor_person_id') OR audit_context->>'actor_person_id' = med_tracker.current_person_id()::text)
  AND (NOT (audit_context ? 'actor_membership_id') OR audit_context->>'actor_membership_id' = actor_membership_id::text)
  AND (NOT (audit_context ? 'household_id') OR audit_context->>'household_id' = household_id::text));

CREATE POLICY versions_subject_global_returning ON public.versions FOR SELECT TO med_tracker_app
USING (household_id IS NULL AND actor_membership_id IS NULL
  AND item_id = med_tracker.current_account_id()
  AND whodunnit = med_tracker.current_person_id()::text
  AND audit_context->>'actor_account_id' = med_tracker.current_account_id()::text
  AND audit_context->>'actor_person_id' = med_tracker.current_person_id()::text
  AND NOT (audit_context ? 'household_id') AND NOT (audit_context ? 'actor_membership_id')
  AND ((item_type = 'Account' AND event IN ('update', 'mobile_oauth.consent_granted',
      'mobile_oauth.token_revoked', 'mobile_oauth.revoked'))
    OR (item_type = 'AuthenticationToken' AND event LIKE 'auth_token/%/%')));

CREATE POLICY versions_household_insert ON public.versions FOR INSERT TO med_tracker_app
WITH CHECK (household_id = med_tracker.current_household_id()
  AND actor_membership_id = med_tracker.current_membership_id()
  AND whodunnit = med_tracker.current_person_id()::text
  AND (item_type NOT IN ('Account', 'AuthenticationToken') OR item_id = med_tracker.current_account_id())
  AND EXISTS (
    SELECT 1 FROM public.household_memberships member
    WHERE member.id = actor_membership_id AND member.household_id = public.versions.household_id
      AND member.person_id = med_tracker.current_person_id()
      AND member.status = 'active' AND member.revoked_at IS NULL
  )
  AND (NOT (audit_context ? 'actor_account_id') OR audit_context->>'actor_account_id' = med_tracker.current_account_id()::text)
  AND (NOT (audit_context ? 'actor_person_id') OR audit_context->>'actor_person_id' = med_tracker.current_person_id()::text)
  AND (NOT (audit_context ? 'actor_membership_id') OR audit_context->>'actor_membership_id' = actor_membership_id::text)
  AND (NOT (audit_context ? 'household_id') OR audit_context->>'household_id' = household_id::text));

CREATE POLICY versions_subject_global_insert ON public.versions FOR INSERT TO med_tracker_app
WITH CHECK (household_id IS NULL AND actor_membership_id IS NULL
  AND item_id = med_tracker.current_account_id()
  AND whodunnit = med_tracker.current_person_id()::text
  AND audit_context->>'actor_account_id' = med_tracker.current_account_id()::text
  AND audit_context->>'actor_person_id' = med_tracker.current_person_id()::text
  AND NOT (audit_context ? 'household_id') AND NOT (audit_context ? 'actor_membership_id')
  AND ((item_type = 'Account' AND event IN ('update', 'mobile_oauth.consent_granted',
      'mobile_oauth.token_revoked', 'mobile_oauth.revoked'))
    OR (item_type = 'AuthenticationToken' AND event LIKE 'auth_token/%/%')));

CREATE FUNCTION med_tracker.can_read_person(target_person bigint) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$
  SELECT target_person = med_tracker.current_person_id() OR EXISTS (
    SELECT 1 FROM public.household_memberships membership
    WHERE membership.person_id = target_person
      AND membership.household_id = med_tracker.current_household_id()
      AND membership.status = 'active' AND membership.revoked_at IS NULL
  )
$$;

CREATE FUNCTION med_tracker.person_display_name(target_person bigint) RETURNS text
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$
  SELECT COALESCE(account.name, person.clinical_name)
  FROM public.people person
  LEFT JOIN public.accounts account ON account.person_id = person.id AND account.status <> 3
  WHERE person.id = target_person AND med_tracker.can_read_person(person.id)
$$;

CREATE POLICY account_self_read ON public.accounts FOR SELECT TO med_tracker_app
USING (id = med_tracker.current_account_id());

CREATE TABLE public.account_avatars (
  account_id bigint PRIMARY KEY REFERENCES public.accounts(id),
  key text NOT NULL UNIQUE,
  content_type text NOT NULL,
  service_name text NOT NULL,
  byte_size bigint NOT NULL CHECK(byte_size > 0),
  checksum text NOT NULL,
  created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);
ALTER TABLE public.account_avatars OWNER TO med_tracker_owner;
ALTER TABLE public.account_avatars ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.account_avatars FORCE ROW LEVEL SECURITY;
GRANT SELECT,INSERT,UPDATE,DELETE ON public.account_avatars TO med_tracker_app;
CREATE POLICY account_avatar_owner ON public.account_avatars TO med_tracker_owner USING (true) WITH CHECK (true);
CREATE POLICY account_avatar_self ON public.account_avatars TO med_tracker_app
USING (account_id=med_tracker.current_account_id() AND EXISTS(SELECT 1 FROM public.accounts a WHERE a.id=account_id AND a.status=2 AND a.disabled_at IS NULL)
  AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=account_avatars.account_id AND l.deadline>CURRENT_TIMESTAMP))
WITH CHECK (account_id=med_tracker.current_account_id() AND EXISTS(SELECT 1 FROM public.accounts a WHERE a.id=account_id AND a.status=2 AND a.disabled_at IS NULL)
  AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=account_avatars.account_id AND l.deadline>CURRENT_TIMESTAMP));

CREATE FUNCTION med_tracker.person_avatar(target_person bigint)
RETURNS TABLE(key text, content_type text, service_name text)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$
  SELECT avatar.key,avatar.content_type,avatar.service_name FROM public.account_avatars avatar
  JOIN public.accounts account ON account.id=avatar.account_id
  WHERE account.person_id=target_person AND account.status=2 AND med_tracker.can_read_person(target_person)
$$;
REVOKE ALL ON FUNCTION med_tracker.person_avatar(bigint) FROM PUBLIC, med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.person_avatar(bigint) TO med_tracker_app;

CREATE FUNCTION med_tracker.avatar_key_retained(target_key text) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$ SELECT EXISTS(SELECT 1 FROM public.account_avatars WHERE key=target_key) $$;
REVOKE ALL ON FUNCTION med_tracker.avatar_key_retained(text) FROM PUBLIC, med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.avatar_key_retained(text) TO med_tracker_app;

CREATE TABLE public.account_profile_events (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id bigint NOT NULL REFERENCES public.accounts(id),
  actor_person_id bigint NOT NULL REFERENCES public.people(id),
  event_type text NOT NULL,
  request_id text NOT NULL,
  metadata jsonb NOT NULL DEFAULT '{}',
  created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);
ALTER TABLE public.account_profile_events OWNER TO med_tracker_owner;
ALTER TABLE public.account_profile_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.account_profile_events FORCE ROW LEVEL SECURITY;
GRANT SELECT,INSERT ON public.account_profile_events TO med_tracker_app;
GRANT USAGE ON SEQUENCE public.account_profile_events_id_seq TO med_tracker_app;
CREATE POLICY profile_event_self ON public.account_profile_events TO med_tracker_app
USING(account_id=med_tracker.current_account_id())
WITH CHECK(account_id=med_tracker.current_account_id() AND actor_person_id=med_tracker.current_person_id());

CREATE FUNCTION med_tracker.household_accounts(target_household bigint)
RETURNS TABLE(account_id bigint, person_id bigint, name text, created_at timestamp, updated_at timestamp)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$
  SELECT account.id, account.person_id, account.name, account.created_at, account.updated_at
  FROM public.accounts account
  JOIN public.household_memberships membership ON membership.person_id = account.person_id
  WHERE membership.household_id = target_household
    AND target_household = med_tracker.current_household_id()
    AND membership.status = 'active' AND membership.revoked_at IS NULL
    AND account.status = 2
$$;
REVOKE ALL ON FUNCTION med_tracker.household_accounts(bigint) FROM PUBLIC, med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.household_accounts(bigint) TO med_tracker_app;

CREATE FUNCTION med_tracker.invitation_historical_inviter(target_invitation bigint) RETURNS bigint
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$
  SELECT account.id
  FROM public.household_invitations invitation
  JOIN public.household_memberships inviter ON inviter.id=invitation.invited_by_membership_id AND inviter.household_id=invitation.household_id
  JOIN public.accounts account ON account.person_id=inviter.person_id
  WHERE invitation.id=target_invitation
    AND invitation.household_id=med_tracker.current_household_id()
    AND EXISTS(SELECT 1 FROM public.household_memberships actor
      WHERE actor.household_id=invitation.household_id AND actor.person_id=med_tracker.current_person_id()
        AND actor.status='active' AND actor.revoked_at IS NULL AND actor.role IN ('owner','administrator'))
$$;
REVOKE ALL ON FUNCTION med_tracker.invitation_historical_inviter(bigint) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.invitation_historical_inviter(bigint) TO med_tracker_app;

CREATE FUNCTION med_tracker.platform_account_state(target_account bigint)
RETURNS TABLE(status integer, disabled boolean, locked boolean)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$
  SELECT target.status,target.disabled_at IS NOT NULL,
    EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=target.id AND l.deadline>CURRENT_TIMESTAMP)
  FROM public.accounts target
  WHERE target.id=target_account AND EXISTS(
    SELECT 1 FROM public.platform_admins administrator
    JOIN public.accounts actor ON actor.id=administrator.account_id
    WHERE actor.id=med_tracker.current_account_id() AND actor.status=2 AND actor.disabled_at IS NULL
      AND administrator.status='active'
      AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=actor.id AND l.deadline>CURRENT_TIMESTAMP))
$$;
REVOKE ALL ON FUNCTION med_tracker.platform_account_state(bigint) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.platform_account_state(bigint) TO med_tracker_app;

CREATE FUNCTION med_tracker.person_avatar_digest(target_person bigint) RETURNS text
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$
  SELECT encode(public.digest(convert_to(lower(btrim(account.email::text)), 'UTF8'), 'sha256'), 'hex')
  FROM public.accounts account
  WHERE account.person_id = target_person AND account.status = 2
    AND account.preferences->>'gravatar_enabled' IN ('true', '1')
    AND med_tracker.can_read_person(target_person)
$$;
REVOKE ALL ON FUNCTION med_tracker.person_avatar_digest(bigint) FROM PUBLIC, med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.person_avatar_digest(bigint) TO med_tracker_app;
CREATE POLICY account_authentication_lookup ON public.accounts FOR SELECT TO med_tracker_app
USING (email = NULLIF(current_setting('med_tracker.identity_email', true), '')::public.citext);
CREATE POLICY account_self_update ON public.accounts FOR UPDATE TO med_tracker_app
USING (id = med_tracker.current_account_id()) WITH CHECK (id = med_tracker.current_account_id());
CREATE POLICY person_clinical_read ON public.people FOR SELECT TO med_tracker_app
USING (med_tracker.can_read_person(id));
CREATE POLICY person_self_update ON public.people FOR UPDATE TO med_tracker_app
USING (id = med_tracker.current_person_id()) WITH CHECK (id = med_tracker.current_person_id());
CREATE POLICY participation_read ON public.household_memberships FOR SELECT TO med_tracker_app
USING (person_id = med_tracker.current_person_id() OR household_id = med_tracker.current_household_id());
CREATE POLICY household_member_read ON public.households FOR SELECT TO med_tracker_app
USING (med_tracker.has_household_access(id));

REVOKE INSERT, DELETE, UPDATE ON public.accounts FROM med_tracker_app;
GRANT UPDATE (name, professional_title, email, password_hash, preferences, status, updated_at) ON public.accounts TO med_tracker_app;
REVOKE INSERT, DELETE ON public.people FROM med_tracker_app;
REVOKE INSERT, UPDATE, DELETE ON public.household_memberships, public.households FROM med_tracker_app;

CREATE FUNCTION med_tracker.enforce_account_person() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
DECLARE kind integer;
BEGIN
  IF TG_OP = 'UPDATE' AND NEW.person_id IS DISTINCT FROM OLD.person_id THEN
    RAISE EXCEPTION 'Account person identity is immutable' USING ERRCODE = '23514';
  END IF;
  UPDATE public.people SET id = id WHERE id = NEW.person_id RETURNING person_type INTO kind;
  IF NOT FOUND THEN
    RAISE EXCEPTION 'Account requires an existing person' USING ERRCODE = '23503';
  END IF;
  IF kind = 1 THEN
    RAISE EXCEPTION 'Minors cannot have accounts' USING ERRCODE = '23514';
  END IF;
  RETURN NEW;
END
$$;
CREATE TRIGGER account_person_guard BEFORE INSERT OR UPDATE OF person_id ON public.accounts
FOR EACH ROW EXECUTE FUNCTION med_tracker.enforce_account_person();

CREATE FUNCTION med_tracker.enforce_person_account() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
BEGIN
  IF NEW.person_type = 1 AND EXISTS (SELECT 1 FROM public.accounts WHERE person_id = NEW.id) THEN
    RAISE EXCEPTION 'A person with an account cannot become a minor' USING ERRCODE = '23514';
  END IF;
  RETURN NEW;
END
$$;
CREATE TRIGGER person_account_guard BEFORE UPDATE OF person_type ON public.people
FOR EACH ROW EXECUTE FUNCTION med_tracker.enforce_person_account();

CREATE FUNCTION med_tracker.enforce_participation_identity() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
BEGIN
  IF TG_OP = 'DELETE' THEN
    RAISE EXCEPTION 'Household participation is retained; revoke it instead' USING ERRCODE = '23514';
  END IF;
  IF NEW.id IS DISTINCT FROM OLD.id OR NEW.person_id IS DISTINCT FROM OLD.person_id
    OR NEW.household_id IS DISTINCT FROM OLD.household_id THEN
    RAISE EXCEPTION 'Household participation identity is immutable' USING ERRCODE = '23514';
  END IF;
  RETURN NEW;
END
$$;
CREATE TRIGGER participation_identity_guard BEFORE UPDATE OR DELETE ON public.household_memberships
FOR EACH ROW EXECUTE FUNCTION med_tracker.enforce_participation_identity();

CREATE FUNCTION med_tracker.preserve_clinical_name() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
BEGIN
  IF TG_OP = 'DELETE' OR NEW.status = 3 THEN
    IF OLD.name IS NOT NULL THEN
      UPDATE public.people SET clinical_name = OLD.name, updated_at = CURRENT_TIMESTAMP WHERE id = OLD.person_id;
    END IF;
  END IF;
  IF TG_OP = 'DELETE' THEN RETURN OLD; END IF;
  RETURN NEW;
END
$$;
CREATE TRIGGER preserve_clinical_name BEFORE UPDATE OR DELETE ON public.accounts
FOR EACH ROW EXECUTE FUNCTION med_tracker.preserve_clinical_name();

CREATE FUNCTION med_tracker.require_person_name() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
DECLARE target_person bigint;
BEGIN
  IF TG_TABLE_NAME = 'people' THEN
    target_person := NEW.id;
  ELSIF TG_OP = 'DELETE' THEN
    target_person := OLD.person_id;
  ELSE
    target_person := NEW.person_id;
  END IF;
  IF EXISTS (SELECT 1 FROM public.people person WHERE person.id = target_person
    AND person.clinical_name IS NULL AND NOT EXISTS (
      SELECT 1 FROM public.accounts account WHERE account.person_id = person.id
      AND account.status <> 3 AND account.name IS NOT NULL AND btrim(account.name) <> ''
    )) THEN
    RAISE EXCEPTION 'Every clinical person requires a display name' USING ERRCODE = '23514';
  END IF;
  RETURN NULL;
END
$$;
CREATE CONSTRAINT TRIGGER person_name_guard AFTER INSERT OR UPDATE ON public.people
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION med_tracker.require_person_name();
CREATE CONSTRAINT TRIGGER account_name_guard AFTER INSERT OR UPDATE OR DELETE ON public.accounts
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION med_tracker.require_person_name();

CREATE FUNCTION med_tracker.bootstrap_account(login_email text, profile_name text, birth date, password_hash text)
RETURNS TABLE(account_id bigint, person_id bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
DECLARE new_person bigint; new_account bigint;
BEGIN
  IF login_email IS NULL OR btrim(login_email) = '' OR position('@' IN login_email) < 2
    OR profile_name IS NULL OR btrim(profile_name) = ''
    OR birth IS NULL OR birth > (CURRENT_DATE - interval '18 years')::date THEN
    RAISE EXCEPTION 'A new account requires an adult identity, email and profile name' USING ERRCODE = '23514';
  END IF;
  INSERT INTO public.people(date_of_birth, person_type, has_capacity, created_at, updated_at)
  VALUES (birth, 0, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) RETURNING id INTO new_person;
  INSERT INTO public.accounts(person_id, name, email, password_hash, status, created_at, updated_at)
  VALUES (new_person, btrim(profile_name), btrim(login_email), password_hash, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
  RETURNING id INTO new_account;
  RETURN QUERY SELECT new_account, new_person;
END
$$;

CREATE FUNCTION med_tracker.create_household(household_name text, household_slug text, household_timezone text, trace_id text)
RETURNS TABLE(household_id bigint, membership_id bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
DECLARE actor_person bigint; new_household bigint; new_membership bigint; actor_context jsonb;
BEGIN
  SELECT account.person_id INTO actor_person FROM public.accounts account
  JOIN public.people person ON person.id = account.person_id
  WHERE account.id = med_tracker.current_account_id() AND account.status = 2 AND account.disabled_at IS NULL
    AND person.person_type = 0 AND person.has_capacity
    AND NOT EXISTS (SELECT 1 FROM public.account_lockouts lockout
      WHERE lockout.account_id = account.id AND lockout.deadline > CURRENT_TIMESTAMP)
  FOR UPDATE OF account, person;
  IF NOT FOUND THEN
    RAISE EXCEPTION 'Household creation requires a current adult account' USING ERRCODE = 'MT001';
  END IF;
  IF household_name IS NULL OR btrim(household_name) = ''
    OR household_slug IS NULL OR btrim(household_slug) = ''
    OR NOT EXISTS (SELECT 1 FROM pg_catalog.pg_timezone_names WHERE name = household_timezone) THEN
    RAISE EXCEPTION 'Household requires name, slug and timezone' USING ERRCODE = '23514';
  END IF;
  INSERT INTO public.households(name, slug, timezone, created_by_account_id, created_at, updated_at)
  VALUES (btrim(household_name), household_slug, household_timezone, med_tracker.current_account_id(), CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
  RETURNING id INTO new_household;
  INSERT INTO public.household_memberships(person_id, household_id, role, status, permissions_version, joined_at, created_at, updated_at)
  VALUES (actor_person, new_household, 'owner', 'active', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
  RETURNING id INTO new_membership;
  actor_context := jsonb_build_object('actor_account_id',med_tracker.current_account_id(),'actor_person_id',actor_person,'actor_membership_id',new_membership,'household_id',new_household,'request_id',trace_id);
  INSERT INTO public.security_audit_events(household_id,actor_account_id,actor_membership_id,event_type,request_id,metadata,audit_context,created_at,updated_at)
  VALUES(new_household,med_tracker.current_account_id(),new_membership,'household.created',trace_id,jsonb_build_object('role','owner'),actor_context,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP);
  INSERT INTO public.api_change_events(household_id,household_membership_id,account_id,action,record_type,record_id,request_id,metadata,occurred_at,created_at,updated_at)
  VALUES(new_household,new_membership,med_tracker.current_account_id(),'create','Household',new_household,trace_id,actor_context,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP);
  RETURN QUERY SELECT new_household, new_membership;
END
$$;

CREATE POLICY registration_audit_insert ON public.security_audit_events FOR INSERT TO med_tracker_owner WITH CHECK (true);
CREATE POLICY registration_change_insert ON public.api_change_events FOR INSERT TO med_tracker_owner WITH CHECK (true);

CREATE POLICY invitation_domain_owner ON public.household_invitations TO med_tracker_owner USING (true) WITH CHECK (true);

CREATE FUNCTION med_tracker.complete_household_invitation(invitation_digest text, trace_id text, new_registration boolean)
RETURNS TABLE(household_id bigint, membership_id bigint, person_id bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
DECLARE invitation public.household_invitations%ROWTYPE; recipient public.accounts%ROWTYPE;
  target_household bigint; actor_person public.people%ROWTYPE; inviter public.household_memberships%ROWTYPE;
  participation public.household_memberships%ROWTYPE; actor_context jsonb;
BEGIN
  SELECT i.household_id INTO target_household FROM public.household_invitations i WHERE i.token_digest = invitation_digest;
  PERFORM h.id FROM public.households h WHERE h.id = target_household AND h.status = 'active' AND h.lifecycle_state = 'active' FOR UPDATE;
  IF NOT FOUND THEN RAISE EXCEPTION 'Invitation unavailable' USING ERRCODE = 'MT001'; END IF;
  SELECT * INTO recipient FROM public.accounts a WHERE a.id = med_tracker.current_account_id() AND a.disabled_at IS NULL AND (a.status = 2 OR (new_registration AND a.status = 1))
    AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id = a.id AND l.deadline > CURRENT_TIMESTAMP) FOR UPDATE;
  IF NOT FOUND THEN RAISE EXCEPTION 'Invitation unavailable' USING ERRCODE = 'MT001'; END IF;
  SELECT * INTO actor_person FROM public.people p WHERE p.id = recipient.person_id FOR UPDATE;
  IF actor_person.person_type <> 0 OR NOT actor_person.has_capacity THEN
    RAISE EXCEPTION 'Existing person sharing requires self or established guardian approval' USING ERRCODE = 'MT001';
  END IF;
  SELECT * INTO invitation FROM public.household_invitations i WHERE i.token_digest = invitation_digest FOR UPDATE;
  IF invitation.email <> recipient.email OR invitation.revoked_at IS NOT NULL
    OR invitation.expires_at <= CURRENT_TIMESTAMP OR invitation.membership_role NOT IN ('member','administrator','owner') THEN
    RAISE EXCEPTION 'Invitation unavailable' USING ERRCODE = 'MT001';
  END IF;
  SELECT * INTO participation FROM public.household_memberships m
    WHERE m.household_id = target_household AND m.person_id = recipient.person_id FOR UPDATE;
  IF invitation.accepted_at IS NOT NULL THEN
    IF participation.id IS NULL OR participation.status <> 'active' OR participation.revoked_at IS NOT NULL THEN
      RAISE EXCEPTION 'Invitation unavailable' USING ERRCODE = 'MT001';
    END IF;
    RETURN QUERY SELECT target_household,participation.id,recipient.person_id;
    RETURN;
  END IF;
  SELECT * INTO inviter FROM public.household_memberships m
    WHERE m.id = invitation.invited_by_membership_id AND m.household_id = target_household
      AND m.status = 'active' AND m.revoked_at IS NULL AND m.role IN ('owner','administrator') FOR UPDATE;
  IF NOT FOUND OR NOT EXISTS(SELECT 1 FROM public.accounts a WHERE a.person_id = inviter.person_id AND a.status = 2 AND a.disabled_at IS NULL
      AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>CURRENT_TIMESTAMP))
    OR (invitation.membership_role = 'owner' AND inviter.role <> 'owner') THEN
    RAISE EXCEPTION 'Invitation unavailable' USING ERRCODE = 'MT001';
  END IF;
  IF participation.id IS NOT NULL THEN
    IF participation.status = 'active' AND participation.revoked_at IS NULL THEN
      RAISE EXCEPTION 'Person already participates' USING ERRCODE = '23505';
    END IF;
    UPDATE public.household_memberships m SET status='active',revoked_at=NULL,role=invitation.membership_role,
      permissions_version=m.permissions_version+1,updated_at=CURRENT_TIMESTAMP WHERE m.id=participation.id;
  ELSE
    INSERT INTO public.household_memberships(person_id,household_id,role,status,permissions_version,joined_at,created_at,updated_at)
    VALUES(recipient.person_id,target_household,invitation.membership_role,'active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)
    RETURNING * INTO participation;
  END IF;
  UPDATE public.household_invitations i SET accepted_at=CURRENT_TIMESTAMP,updated_at=CURRENT_TIMESTAMP WHERE i.id=invitation.id;
  actor_context := jsonb_build_object('actor_account_id',recipient.id,'actor_person_id',recipient.person_id,'actor_membership_id',participation.id,'household_id',target_household,'request_id',trace_id);
  INSERT INTO public.security_audit_events(household_id,actor_account_id,actor_membership_id,event_type,request_id,metadata,audit_context,created_at,updated_at)
  VALUES(target_household,recipient.id,participation.id,'household_invitation.accepted',trace_id,
    jsonb_build_object('invitation_id',invitation.id,'person_id',recipient.person_id,'invited_by_membership_id',inviter.id),actor_context,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP);
  INSERT INTO public.api_change_events(household_id,household_membership_id,account_id,action,record_type,record_id,request_id,metadata,occurred_at,created_at,updated_at)
  VALUES(target_household,participation.id,recipient.id,'create','HouseholdMembership',participation.id,trace_id,actor_context,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP);
  RETURN QUERY SELECT target_household,participation.id,recipient.person_id;
END
$$;
REVOKE ALL ON FUNCTION med_tracker.complete_household_invitation(text,text,boolean) FROM PUBLIC, med_tracker_app;
CREATE FUNCTION med_tracker.invitation_preview(invitation_digest text)
RETURNS TABLE(email text, membership_role text, household_name text)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$
  SELECT i.email::text,i.membership_role::text,h.name::text
  FROM public.household_invitations i JOIN public.households h ON h.id=i.household_id
  JOIN public.household_memberships m ON m.id=i.invited_by_membership_id AND m.household_id=h.id
  JOIN public.accounts a ON a.person_id=m.person_id
  WHERE i.token_digest=invitation_digest AND i.accepted_at IS NULL AND i.revoked_at IS NULL AND i.expires_at>CURRENT_TIMESTAMP
    AND h.status='active' AND h.lifecycle_state='active'
    AND m.status='active' AND m.revoked_at IS NULL AND m.role IN ('owner','administrator')
    AND a.status=2 AND a.disabled_at IS NULL AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>CURRENT_TIMESTAMP)
$$;
REVOKE ALL ON FUNCTION med_tracker.invitation_preview(text) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.invitation_preview(text) TO med_tracker_app;

CREATE FUNCTION med_tracker.accept_household_invitation(invitation_digest text, trace_id text)
RETURNS TABLE(household_id bigint, membership_id bigint, person_id bigint)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog
AS $$ SELECT * FROM med_tracker.complete_household_invitation(invitation_digest,trace_id,false) $$;
REVOKE ALL ON FUNCTION med_tracker.accept_household_invitation(text,text) FROM PUBLIC, med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.accept_household_invitation(text,text) TO med_tracker_app;

CREATE FUNCTION med_tracker.bootstrap_invited_registration(invitation_digest text, profile_name text, birth date, password_hash text, trace_id text)
RETURNS TABLE(account_id bigint, person_id bigint, household_id bigint, membership_id bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
DECLARE invitation_email text; identity record; accepted record;
BEGIN
  SELECT i.email INTO invitation_email FROM public.household_invitations i
    WHERE i.token_digest=invitation_digest AND i.accepted_at IS NULL AND i.revoked_at IS NULL AND i.expires_at>CURRENT_TIMESTAMP;
  IF NOT FOUND THEN RAISE EXCEPTION 'Invitation unavailable' USING ERRCODE = '42501'; END IF;
  SELECT * INTO identity FROM med_tracker.bootstrap_account(invitation_email,profile_name,birth,password_hash);
  PERFORM set_config('med_tracker.current_account_id',identity.account_id::text,true);
  SELECT * INTO accepted FROM med_tracker.complete_household_invitation(invitation_digest,trace_id,true);
  RETURN QUERY SELECT identity.account_id,identity.person_id,accepted.household_id,accepted.membership_id;
END
$$;
REVOKE ALL ON FUNCTION med_tracker.bootstrap_invited_registration(text,text,date,text,text) FROM PUBLIC, med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.bootstrap_invited_registration(text,text,date,text,text) TO med_tracker_app;

CREATE FUNCTION med_tracker.update_household_settings(requested_name text, requested_timezone text, requested_plan text)
RETURNS SETOF public.households
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
DECLARE selected bigint;
BEGIN
  PERFORM med_tracker.lock_current_household();
  selected := med_tracker.current_household_id();
  IF NOT EXISTS(SELECT 1 FROM public.household_memberships m WHERE m.household_id=selected
    AND m.person_id=med_tracker.current_person_id() AND m.status='active' AND m.revoked_at IS NULL
    AND m.role IN ('owner','administrator')) THEN
    RAISE EXCEPTION 'Current manager required' USING ERRCODE='42501';
  END IF;
  IF requested_name IS NULL OR btrim(requested_name)='' OR requested_plan IS NULL OR requested_plan NOT IN ('free','family_plus')
    OR requested_timezone IS NULL OR NOT EXISTS(SELECT 1 FROM pg_catalog.pg_timezone_names WHERE name=requested_timezone) THEN
    RAISE EXCEPTION 'Invalid household settings' USING ERRCODE='23514';
  END IF;
  RETURN QUERY UPDATE public.households h SET name=btrim(requested_name),timezone=requested_timezone,
    subscription_plan=requested_plan,updated_at=CURRENT_TIMESTAMP WHERE h.id=selected RETURNING h.*;
END
$$;
REVOKE ALL ON FUNCTION med_tracker.update_household_settings(text,text,text) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.update_household_settings(text,text,text) TO med_tracker_app;

CREATE FUNCTION med_tracker.change_household_membership(target_membership bigint, requested_role text, requested_status text, trace_id text, credential_context jsonb DEFAULT '{}'::jsonb)
RETURNS SETOF public.household_memberships
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
DECLARE selected bigint; actor public.household_memberships%ROWTYPE; previous public.household_memberships%ROWTYPE;
  changed public.household_memberships%ROWTYPE; was_owner boolean; remains_owner boolean; actor_context jsonb;
BEGIN
  PERFORM med_tracker.lock_current_household();
  selected := med_tracker.current_household_id();
  SELECT * INTO actor FROM public.household_memberships m WHERE m.household_id=selected
    AND m.person_id=med_tracker.current_person_id() AND m.status='active' AND m.revoked_at IS NULL FOR UPDATE;
  IF NOT FOUND OR actor.role NOT IN ('owner','administrator') THEN
    RAISE EXCEPTION 'Current manager required' USING ERRCODE='42501';
  END IF;
  SELECT * INTO previous FROM public.household_memberships m WHERE m.id=target_membership AND m.household_id=selected FOR UPDATE;
  IF NOT FOUND THEN RAISE EXCEPTION 'Participation unavailable' USING ERRCODE='42501'; END IF;
  IF requested_role IS NULL OR requested_role NOT IN ('owner','administrator','member')
    OR requested_status IS NULL OR requested_status NOT IN ('active','suspended','revoked') THEN
    RAISE EXCEPTION 'Invalid participation state' USING ERRCODE='23514';
  END IF;
  was_owner := previous.role='owner' AND previous.status='active' AND previous.revoked_at IS NULL;
  remains_owner := requested_role='owner' AND requested_status='active';
  IF was_owner <> remains_owner AND (remains_owner OR actor.role <> 'owner') THEN
    RAISE EXCEPTION 'Owner change unavailable' USING ERRCODE='42501';
  END IF;
  IF was_owner AND NOT remains_owner AND NOT EXISTS(
    SELECT 1 FROM public.household_memberships m JOIN public.accounts a ON a.person_id=m.person_id
    WHERE m.household_id=selected AND m.id<>target_membership AND m.role='owner' AND m.status='active' AND m.revoked_at IS NULL
      AND a.status=2 AND a.disabled_at IS NULL AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>CURRENT_TIMESTAMP)) THEN
    RAISE EXCEPTION 'Last usable owner cannot be removed' USING ERRCODE='23514';
  END IF;
  IF previous.role=requested_role AND previous.status=requested_status
    AND ((requested_status='revoked' AND previous.revoked_at IS NOT NULL) OR (requested_status<>'revoked' AND previous.revoked_at IS NULL)) THEN
    RETURN NEXT previous; RETURN;
  END IF;
  UPDATE public.household_memberships m SET role=requested_role,status=requested_status,
    revoked_at=CASE WHEN requested_status='revoked' THEN COALESCE(m.revoked_at,CURRENT_TIMESTAMP) ELSE NULL END,
    permissions_version=m.permissions_version+1,updated_at=CURRENT_TIMESTAMP WHERE m.id=target_membership RETURNING * INTO changed;
  actor_context := COALESCE(credential_context,'{}'::jsonb) || jsonb_build_object('actor_account_id',med_tracker.current_account_id(),
    'actor_person_id',actor.person_id,'actor_membership_id',actor.id,'household_id',selected,'request_id',trace_id);
  INSERT INTO public.security_audit_events(household_id,actor_account_id,actor_membership_id,event_type,request_id,metadata,audit_context,created_at,updated_at)
  VALUES(selected,med_tracker.current_account_id(),actor.id,
    CASE WHEN previous.role<>changed.role THEN 'household_membership.role_updated' ELSE 'household_access.membership_changed' END,
    trace_id,jsonb_build_object('target_person_id',previous.person_id,'target_membership_id',previous.id,
      'previous_state',to_jsonb(previous),'new_state',to_jsonb(changed),'outcome','success'),actor_context,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP);
  INSERT INTO public.api_change_events(household_id,household_membership_id,account_id,action,record_type,record_id,request_id,metadata,occurred_at,created_at,updated_at)
  VALUES(selected,actor.id,med_tracker.current_account_id(),'update','HouseholdMembership',changed.id,trace_id,actor_context,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP);
  RETURN NEXT changed;
END
$$;
REVOKE ALL ON FUNCTION med_tracker.change_household_membership(bigint,text,text,text,jsonb) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.change_household_membership(bigint,text,text,text,jsonb) TO med_tracker_app;

CREATE FUNCTION med_tracker.lock_current_household() RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
DECLARE selected bigint;
BEGIN
  selected := med_tracker.current_household_id();
  IF selected IS NULL OR NOT med_tracker.has_household_access(selected) THEN
    RAISE EXCEPTION 'Current household required' USING ERRCODE='42501';
  END IF;
  PERFORM id FROM public.households WHERE id=selected FOR UPDATE;
  IF NOT FOUND OR NOT med_tracker.has_household_access(selected) THEN
    RAISE EXCEPTION 'Current household required' USING ERRCODE='42501';
  END IF;
END
$$;
REVOKE ALL ON FUNCTION med_tracker.lock_current_household() FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.lock_current_household() TO med_tracker_app;

CREATE FUNCTION med_tracker.can_edit_clinical_person(target_person bigint) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$
  SELECT med_tracker.has_household_access(med_tracker.current_household_id())
    AND EXISTS(SELECT 1 FROM public.household_memberships m WHERE m.household_id=med_tracker.current_household_id()
      AND m.person_id=med_tracker.current_person_id() AND m.status='active' AND m.revoked_at IS NULL AND m.role IN ('owner','administrator'))
    AND EXISTS(SELECT 1 FROM public.household_memberships m WHERE m.household_id=med_tracker.current_household_id()
      AND m.person_id=target_person AND m.status='active' AND m.revoked_at IS NULL)
    AND NOT EXISTS(SELECT 1 FROM public.accounts a WHERE a.person_id=target_person)
$$;
REVOKE ALL ON FUNCTION med_tracker.can_edit_clinical_person(bigint) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.can_edit_clinical_person(bigint) TO med_tracker_app;

CREATE FUNCTION med_tracker.update_clinical_person(target_person bigint, clinical_name text, birth date, kind integer, capacity boolean)
RETURNS SETOF public.people
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
DECLARE selected_household bigint; manager bigint;
BEGIN
  selected_household := med_tracker.current_household_id();
  PERFORM id FROM public.households WHERE id=selected_household FOR UPDATE;
  SELECT m.id INTO manager FROM public.household_memberships m
  WHERE m.household_id=selected_household AND m.person_id=med_tracker.current_person_id()
    AND m.status='active' AND m.revoked_at IS NULL AND m.role IN ('owner','administrator') FOR UPDATE;
  IF NOT FOUND OR NOT med_tracker.has_household_access(selected_household) THEN
    RAISE EXCEPTION 'Clinical editing requires a current manager' USING ERRCODE='42501';
  END IF;
  PERFORM id FROM public.people WHERE id=target_person FOR UPDATE;
  IF NOT FOUND OR NOT med_tracker.can_edit_clinical_person(target_person) THEN
    RAISE EXCEPTION 'Clinical editing requires an accountless current participant' USING ERRCODE='42501';
  END IF;
  IF clinical_name IS NULL OR btrim(clinical_name)='' OR birth IS NULL OR kind IS NULL
    OR kind NOT IN (0,1,2) OR capacity IS NULL OR (kind IN (1,2) AND capacity) THEN
    RAISE EXCEPTION 'Invalid clinical person' USING ERRCODE='23514';
  END IF;
  RETURN QUERY UPDATE public.people p SET clinical_name=btrim(update_clinical_person.clinical_name),
    date_of_birth=birth,person_type=kind,has_capacity=capacity,updated_at=CURRENT_TIMESTAMP
    WHERE p.id=target_person RETURNING p.*;
END
$$;
REVOKE ALL ON FUNCTION med_tracker.update_clinical_person(bigint,text,date,integer,boolean) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.update_clinical_person(bigint,text,date,integer,boolean) TO med_tracker_app;

CREATE FUNCTION med_tracker.create_household_person(clinical_name text, birth date, kind integer, capacity boolean)
RETURNS TABLE(person_id bigint, membership_id bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
DECLARE selected_household bigint; manager bigint; created_person bigint; created_membership bigint;
BEGIN
  selected_household := med_tracker.current_household_id();
  PERFORM id FROM public.households WHERE id = selected_household FOR UPDATE;
  SELECT membership.id INTO manager FROM public.household_memberships membership
  WHERE membership.household_id = selected_household
    AND membership.person_id = med_tracker.current_person_id()
    AND membership.status = 'active' AND membership.revoked_at IS NULL
    AND membership.role IN ('owner', 'administrator') FOR UPDATE;
  IF NOT FOUND OR NOT med_tracker.has_household_access(selected_household) THEN
    RAISE EXCEPTION 'Fresh participation requires a current manager' USING ERRCODE = '42501';
  END IF;
  IF clinical_name IS NULL OR btrim(clinical_name) = '' OR birth IS NULL
    OR kind IS NULL OR kind NOT IN (0,1,2) OR capacity IS NULL
    OR (kind IN (1,2) AND capacity) THEN
    RAISE EXCEPTION 'Invalid clinical person' USING ERRCODE = '23514';
  END IF;
  INSERT INTO public.people(clinical_name,date_of_birth,person_type,has_capacity,created_at,updated_at)
  VALUES(btrim(clinical_name),birth,kind,capacity,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)
  RETURNING id INTO created_person;
  INSERT INTO public.household_memberships(person_id,household_id,role,status,permissions_version,joined_at,created_at,updated_at)
  VALUES(created_person,selected_household,'member','active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)
  RETURNING id INTO created_membership;
  RETURN QUERY SELECT created_person,created_membership;
END
$$;
REVOKE ALL ON FUNCTION med_tracker.create_household_person(text,date,integer,boolean) FROM PUBLIC, med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.create_household_person(text,date,integer,boolean) TO med_tracker_app;

CREATE FUNCTION med_tracker.bootstrap_registration(login_email text, profile_name text, birth date, password_hash text,
  household_name text, household_slug text, household_timezone text, trace_id text)
RETURNS TABLE(account_id bigint, person_id bigint, household_id bigint, membership_id bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog
AS $$
DECLARE identity record; new_household bigint; new_membership bigint; actor_context jsonb;
BEGIN
  IF household_name IS NULL OR btrim(household_name) = ''
    OR household_slug IS NULL OR btrim(household_slug) = ''
    OR NOT EXISTS (SELECT 1 FROM pg_catalog.pg_timezone_names WHERE name = household_timezone) THEN
    RAISE EXCEPTION 'Household requires name, slug and timezone' USING ERRCODE = '23514';
  END IF;
  SELECT * INTO identity FROM med_tracker.bootstrap_account(login_email, profile_name, birth, password_hash);
  INSERT INTO public.households(name, slug, timezone, created_by_account_id, created_at, updated_at)
  VALUES (btrim(household_name), household_slug, household_timezone, identity.account_id, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
  RETURNING id INTO new_household;
  INSERT INTO public.household_memberships(person_id, household_id, role, status, permissions_version, joined_at, created_at, updated_at)
  VALUES (identity.person_id, new_household, 'owner', 'active', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
  RETURNING id INTO new_membership;
  actor_context := jsonb_build_object('actor_account_id', identity.account_id, 'actor_person_id', identity.person_id,
    'actor_membership_id', new_membership, 'household_id', new_household, 'request_id', trace_id);
  INSERT INTO public.security_audit_events(household_id, actor_account_id, actor_membership_id, event_type, metadata, audit_context, request_id, created_at, updated_at)
  VALUES (new_household, identity.account_id, new_membership, 'household_access.membership_created',
    jsonb_build_object('target_account_id',identity.account_id,'target_membership_id',new_membership,
      'previous_state',NULL,'new_state',jsonb_build_object('role','owner','status','active','person_id',identity.person_id,'permissions_version',1),'outcome','success'),
    actor_context, trace_id, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);
  INSERT INTO public.versions(item_type,item_id,event,object_changes,household_id,request_id,audit_context,created_at)
  VALUES ('Person',identity.person_id,'create',jsonb_build_object('person_type',jsonb_build_array(NULL,0),'has_capacity',jsonb_build_array(NULL,true))::text,
    new_household,trace_id,actor_context,CURRENT_TIMESTAMP);
  INSERT INTO public.api_change_events(household_id,account_id,action,record_type,record_id,record_portable_id,request_id,metadata,occurred_at,created_at,updated_at)
  SELECT new_household,identity.account_id,'create','Person',p.id,p.portable_id,trace_id,
    jsonb_build_object('record_type','Person','record_id',p.id,'portable_id',p.portable_id),CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP
  FROM public.people p WHERE p.id=identity.person_id;
  RETURN QUERY SELECT identity.account_id, identity.person_id, new_household, new_membership;
END
$$;

REVOKE ALL ON FUNCTION med_tracker.bootstrap_registration(text, text, date, text, text, text, text, text) FROM PUBLIC, med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.bootstrap_registration(text, text, date, text, text, text, text, text) TO med_tracker_app;

REVOKE ALL ON FUNCTION med_tracker.create_household(text, text, text, text) FROM PUBLIC, med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.create_household(text, text, text, text) TO med_tracker_app;

REVOKE ALL ON FUNCTION med_tracker.current_person_id(), med_tracker.current_household_id(), med_tracker.has_household_access(bigint),
  med_tracker.can_read_person(bigint), med_tracker.person_display_name(bigint),
  med_tracker.enforce_account_person(), med_tracker.enforce_person_account(), med_tracker.enforce_participation_identity(),
  med_tracker.preserve_clinical_name(), med_tracker.require_person_name(),
  med_tracker.bootstrap_account(text, text, date, text) FROM PUBLIC, med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.current_person_id(), med_tracker.current_household_id(), med_tracker.has_household_access(bigint),
  med_tracker.can_read_person(bigint), med_tracker.person_display_name(bigint),
  med_tracker.bootstrap_account(text, text, date, text) TO med_tracker_app;
