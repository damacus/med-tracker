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
AS $$ SELECT person_id FROM public.accounts WHERE id = med_tracker.current_account_id() AND status IN (1, 2) $$;

CREATE FUNCTION med_tracker.has_household_access(target_household bigint) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$
  SELECT EXISTS (
    SELECT 1 FROM public.household_memberships membership
    JOIN public.accounts account ON account.person_id = membership.person_id
    JOIN public.households household ON household.id = membership.household_id
    WHERE membership.household_id = target_household
      AND account.id = med_tracker.current_account_id() AND account.status = 2
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

REVOKE ALL ON FUNCTION med_tracker.current_person_id(), med_tracker.current_household_id(), med_tracker.has_household_access(bigint),
  med_tracker.can_read_person(bigint), med_tracker.person_display_name(bigint),
  med_tracker.enforce_account_person(), med_tracker.enforce_person_account(), med_tracker.enforce_participation_identity(),
  med_tracker.preserve_clinical_name(), med_tracker.require_person_name(),
  med_tracker.bootstrap_account(text, text, date, text) FROM PUBLIC, med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.current_person_id(), med_tracker.current_household_id(), med_tracker.has_household_access(bigint),
  med_tracker.can_read_person(bigint), med_tracker.person_display_name(bigint),
  med_tracker.bootstrap_account(text, text, date, text) TO med_tracker_app;
