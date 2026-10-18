CREATE TABLE public.account_avatars (
  account_id bigint PRIMARY KEY REFERENCES public.accounts(id),
  key text NOT NULL UNIQUE,
  content_type text NOT NULL,
  service_name text NOT NULL,
  byte_size bigint NOT NULL CHECK (byte_size > 0),
  checksum text NOT NULL,
  created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);
ALTER TABLE public.account_avatars OWNER TO med_tracker_owner;
ALTER TABLE public.account_avatars ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.account_avatars FORCE ROW LEVEL SECURITY;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.account_avatars TO med_tracker_app;
CREATE POLICY account_avatar_owner ON public.account_avatars TO med_tracker_owner USING (true) WITH CHECK (true);
CREATE POLICY account_avatar_self ON public.account_avatars TO med_tracker_app
USING (account_id = med_tracker.current_account_id()
  AND EXISTS (SELECT 1 FROM public.accounts a WHERE a.id = account_avatars.account_id AND a.status = 2)
  AND NOT EXISTS (SELECT 1 FROM public.account_lockouts l WHERE l.account_id = account_avatars.account_id AND l.deadline > CURRENT_TIMESTAMP))
WITH CHECK (account_id = med_tracker.current_account_id()
  AND EXISTS (SELECT 1 FROM public.accounts a WHERE a.id = account_avatars.account_id AND a.status = 2)
  AND NOT EXISTS (SELECT 1 FROM public.account_lockouts l WHERE l.account_id = account_avatars.account_id AND l.deadline > CURRENT_TIMESTAMP));

CREATE POLICY account_avatar_depicted_person ON public.account_avatars FOR SELECT TO med_tracker_app
USING (EXISTS (
  SELECT 1 FROM public.accounts depicted
  JOIN public.household_memberships subject ON subject.person_id = depicted.person_id
  JOIN public.household_memberships m ON m.household_id = subject.household_id
  JOIN public.accounts actor ON actor.person_id = m.person_id
  JOIN public.households h ON h.id = subject.household_id
  WHERE depicted.id = account_avatars.account_id AND subject.household_id = med_tracker.current_household_id()
    AND subject.status = 'active' AND subject.revoked_at IS NULL
    AND m.id = med_tracker.current_membership_id() AND actor.id = med_tracker.current_account_id()
    AND m.status = 'active' AND m.revoked_at IS NULL
    AND h.status = 'active' AND h.lifecycle_state = 'active' AND actor.status = 2
    AND NOT EXISTS (SELECT 1 FROM public.account_lockouts l WHERE l.account_id = actor.id AND l.deadline > CURRENT_TIMESTAMP)
));

CREATE FUNCTION public.avatar_key_retained(target_key text) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog
AS $$ SELECT EXISTS (SELECT 1 FROM public.active_storage_blobs WHERE key = target_key)
  OR EXISTS (SELECT 1 FROM public.account_avatars WHERE key = target_key) $$;
ALTER FUNCTION public.avatar_key_retained(text) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION public.avatar_key_retained(text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.avatar_key_retained(text) TO med_tracker_app;
