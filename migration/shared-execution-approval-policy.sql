-- OFFSET 0 keeps the prior-approval lookup correlated so its household and
-- person filters run before unrelated approval rows' RLS checks.
ALTER POLICY shared_execution_identifier_insert ON public.shared_schedule_executions
WITH CHECK (household_id=med_tracker.current_household_id()
 AND EXISTS(SELECT 1 FROM public.schedules source
  WHERE source.id=schedule_id AND source.portable_id=shared_schedule_executions.portable_id
   AND source.household_id=shared_schedule_executions.household_id AND source.person_id=shared_schedule_executions.person_id
   AND source.person_schedule_id=shared_schedule_executions.person_schedule_id
   AND ((approval_id IS NULL AND source.canonical_origin AND source.sharing_approval_id IS NULL
    AND med_tracker.sharing_clinical_authority(source.person_id)
    AND NOT EXISTS(SELECT 1 FROM public.person_household_approvals prior WHERE prior.person_id=source.person_id AND prior.household_id=source.household_id OFFSET 0))
   OR (source.execution_ended_at IS NULL AND med_tracker.sharing_current_member(source.person_id)
   AND (source.sharing_approval_id=approval_id OR (source.sharing_approval_id IS NULL AND source.canonical_origin
    AND med_tracker.sharing_clinical_authority(source.person_id)))
   AND EXISTS(SELECT 1 FROM public.person_household_approvals approval
    WHERE approval.id=approval_id AND approval.person_id=source.person_id
     AND approval.household_id=source.household_id AND approval.ended_at IS NULL)))));
