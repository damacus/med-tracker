ALTER TABLE public.person_schedules OWNER TO med_tracker_owner;
ALTER TABLE public.person_schedules ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.person_schedules FORCE ROW LEVEL SECURITY;
REVOKE ALL ON public.person_schedules FROM PUBLIC,med_tracker_app;
GRANT SELECT,INSERT,UPDATE ON public.person_schedules TO med_tracker_app;

CREATE POLICY person_schedule_account ON public.person_schedules
USING (person_id=(SELECT person_id FROM public.accounts WHERE id=med_tracker.current_account_id() AND status=2))
WITH CHECK (person_id=(SELECT person_id FROM public.accounts WHERE id=med_tracker.current_account_id() AND status=2));

CREATE POLICY person_schedule_household ON public.person_schedules
USING (origin_household_id=med_tracker.current_household_id()
  AND EXISTS(SELECT 1 FROM public.household_memberships actor WHERE actor.id=med_tracker.current_membership_id() AND actor.household_id=origin_household_id AND actor.status='active' AND actor.revoked_at IS NULL)
  AND EXISTS(SELECT 1 FROM public.household_memberships subject WHERE subject.person_id=person_schedules.person_id AND subject.household_id=origin_household_id AND subject.status IN ('active','suspended') AND subject.revoked_at IS NULL))
WITH CHECK (origin_household_id=med_tracker.current_household_id()
  AND EXISTS(SELECT 1 FROM public.household_memberships actor WHERE actor.id=med_tracker.current_membership_id() AND actor.household_id=origin_household_id AND actor.status='active' AND actor.revoked_at IS NULL)
  AND EXISTS(SELECT 1 FROM public.household_memberships subject WHERE subject.person_id=person_schedules.person_id AND subject.household_id=origin_household_id AND subject.status IN ('active','suspended') AND subject.revoked_at IS NULL));
