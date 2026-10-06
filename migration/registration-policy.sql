CREATE POLICY registration_active_owner_lookup ON public.household_memberships
  FOR SELECT TO med_tracker_owner
  USING (role = 'owner' AND status = 'active');

CREATE FUNCTION public.registration_has_active_owner() RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER
  SET search_path = pg_catalog, pg_temp
  AS $$
    SELECT EXISTS (
      SELECT 1 FROM public.household_memberships
      WHERE role = 'owner' AND status = 'active'
    );
  $$;
REVOKE ALL ON FUNCTION public.registration_has_active_owner() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.registration_has_active_owner() TO med_tracker_app;
