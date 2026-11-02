ALTER POLICY person_schedule_household ON public.person_schedules
USING (origin_household_id=med_tracker.current_household_id()
  AND EXISTS(SELECT 1 FROM public.household_memberships actor WHERE actor.id=med_tracker.current_membership_id() AND actor.household_id=origin_household_id AND actor.status='active' AND actor.revoked_at IS NULL)
  AND EXISTS(SELECT 1 FROM public.household_memberships subject WHERE subject.person_id=person_schedules.person_id AND subject.household_id=origin_household_id AND subject.status IN ('active','suspended') AND subject.revoked_at IS NULL)
  AND EXISTS(SELECT 1 FROM public.schedules execution WHERE execution.id=origin_schedule_id AND execution.household_id=origin_household_id AND execution.person_id=person_schedules.person_id AND execution.execution_ended_at IS NULL))
WITH CHECK (origin_household_id=med_tracker.current_household_id()
  AND EXISTS(SELECT 1 FROM public.household_memberships actor WHERE actor.id=med_tracker.current_membership_id() AND actor.household_id=origin_household_id AND actor.status='active' AND actor.revoked_at IS NULL)
  AND EXISTS(SELECT 1 FROM public.household_memberships subject WHERE subject.person_id=person_schedules.person_id AND subject.household_id=origin_household_id AND subject.status IN ('active','suspended') AND subject.revoked_at IS NULL)
  AND EXISTS(SELECT 1 FROM public.schedules execution WHERE execution.id=origin_schedule_id AND execution.household_id=origin_household_id AND execution.person_id=person_schedules.person_id AND execution.execution_ended_at IS NULL));
