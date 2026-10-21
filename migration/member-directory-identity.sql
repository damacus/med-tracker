CREATE FUNCTION med_tracker.can_read_member_directory()
RETURNS boolean LANGUAGE sql VOLATILE SECURITY INVOKER
SET search_path=pg_catalog
AS $body$
  SELECT EXISTS (
    SELECT 1 FROM public.household_memberships actor
    JOIN public.accounts account ON account.person_id=actor.person_id
    JOIN public.households household ON household.id=actor.household_id
    WHERE actor.id=med_tracker.current_membership_id()
      AND actor.household_id=med_tracker.current_household_id()
      AND actor.status='active' AND actor.revoked_at IS NULL
      AND actor.role IN ('owner','administrator')
      AND account.id=med_tracker.current_account_id() AND account.status=2
      AND household.status='active' AND household.lifecycle_state='active'
      AND NOT EXISTS (
        SELECT 1 FROM public.account_lockouts lockout
        WHERE lockout.account_id=account.id
          AND lockout.deadline>pg_catalog.timezone('UTC',pg_catalog.clock_timestamp())
      )
  );
$body$;
ALTER FUNCTION med_tracker.can_read_member_directory() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.can_read_member_directory() FROM PUBLIC,med_tracker_app;

CREATE POLICY people_member_directory_owner ON public.people
FOR SELECT TO med_tracker_owner
USING (
  med_tracker.can_read_member_directory()
  AND EXISTS (
    SELECT 1 FROM public.household_memberships subject
    WHERE subject.person_id=people.id
      AND subject.household_id=med_tracker.current_household_id()
  )
);

CREATE FUNCTION med_tracker.member_directory_identity()
RETURNS TABLE(membership_id bigint,name text,email text)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path=pg_catalog
AS $body$
BEGIN
  IF NOT med_tracker.can_read_member_directory() THEN
    RAISE EXCEPTION USING ERRCODE='42501',MESSAGE='Member directory unavailable';
  END IF;
  RETURN QUERY
    SELECT membership.id,
      coalesce(nullif(account.name,''),account.email,person.name,'Member #'||membership.id::text)::text,
      coalesce(account.email,person.email,'')::text
    FROM public.household_memberships membership
    LEFT JOIN public.accounts account ON account.person_id=membership.person_id
    LEFT JOIN public.people person ON person.id=membership.person_id
    WHERE membership.household_id=med_tracker.current_household_id();
END;
$body$;
ALTER FUNCTION med_tracker.member_directory_identity() OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.member_directory_identity() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION med_tracker.member_directory_identity() TO med_tracker_app;
