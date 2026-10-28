CREATE POLICY people_recovery_eligibility_owner ON public.people
FOR SELECT TO med_tracker_owner
USING (
  EXISTS (
    SELECT 1 FROM public.household_memberships subject
    WHERE subject.person_id = people.id
      AND subject.household_id = med_tracker.current_household_id()
      AND subject.status = 'active' AND subject.revoked_at IS NULL
  )
  AND EXISTS (
    SELECT 1 FROM public.accounts actor
    JOIN public.platform_admins administrator ON administrator.account_id = actor.id
    JOIN public.households household ON household.id = med_tracker.current_household_id()
    WHERE actor.id = med_tracker.current_account_id()
      AND actor.status = 2 AND actor.active AND administrator.status = 'active'
      AND household.status = 'active' AND household.lifecycle_state = 'active'
      AND NOT EXISTS (
        SELECT 1 FROM public.account_lockouts lockout
        WHERE lockout.account_id = actor.id
          AND lockout.deadline > pg_catalog.timezone('UTC',pg_catalog.clock_timestamp())
      )
  )
);

CREATE FUNCTION med_tracker.recovery_member_eligible(p_membership_id bigint)
RETURNS boolean
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM public.accounts actor
    JOIN public.platform_admins administrator ON administrator.account_id = actor.id
    JOIN public.households household ON household.id = med_tracker.current_household_id()
    WHERE actor.id = med_tracker.current_account_id()
      AND actor.status = 2 AND actor.active AND administrator.status = 'active'
      AND household.status = 'active' AND household.lifecycle_state = 'active'
      AND NOT EXISTS (
        SELECT 1 FROM public.account_lockouts lockout
        WHERE lockout.account_id = actor.id
          AND lockout.deadline > pg_catalog.timezone('UTC',pg_catalog.clock_timestamp())
      )
  ) THEN
    RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Recovery eligibility unavailable';
  END IF;
  RETURN EXISTS (
    SELECT 1 FROM public.household_memberships membership
    JOIN public.accounts account ON account.person_id = membership.person_id
    JOIN public.people person ON person.id = membership.person_id
    WHERE membership.id = p_membership_id
      AND membership.household_id = med_tracker.current_household_id()
      AND membership.status = 'active' AND membership.revoked_at IS NULL
      AND account.status = 2 AND account.active
      AND person.person_type = 0 AND person.has_capacity
  );
END;
$$;
ALTER FUNCTION med_tracker.recovery_member_eligible(bigint) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.recovery_member_eligible(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION med_tracker.recovery_member_eligible(bigint) TO med_tracker_app;
