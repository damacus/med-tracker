ALTER POLICY shared_schedule_current_execution ON public.schedules
USING ((sharing_approval_id IS NULL AND person_schedule_id IS NULL)
 OR med_tracker.sharing_origin_execution(id,person_schedule_id,person_id,household_id,canonical_origin) OR execution_ended_at IS NOT NULL
 OR (med_tracker.sharing_current_member(person_id) AND (EXISTS(SELECT 1 FROM public.person_household_approvals approval WHERE approval.ended_at IS NULL
  AND approval.id=sharing_approval_id) OR EXISTS(SELECT 1 FROM public.shared_schedule_executions link JOIN public.person_household_approvals approval ON approval.id=link.approval_id
  WHERE link.schedule_id=schedules.id AND approval.ended_at IS NULL)))
 OR (med_tracker.sharing_clinical_authority(person_id) AND (sharing_approval_id::text=current_setting('med_tracker.sharing_withdrawal_approval_id',true)
  OR EXISTS(SELECT 1 FROM public.shared_schedule_executions link WHERE link.schedule_id=schedules.id AND link.approval_id::text=current_setting('med_tracker.sharing_withdrawal_approval_id',true)))))
WITH CHECK ((sharing_approval_id IS NULL AND person_schedule_id IS NULL)
 OR med_tracker.sharing_origin_execution(id,person_schedule_id,person_id,household_id,canonical_origin) OR execution_ended_at IS NOT NULL
 OR (med_tracker.sharing_current_member(person_id) AND (EXISTS(SELECT 1 FROM public.person_household_approvals approval WHERE approval.ended_at IS NULL
  AND approval.id=sharing_approval_id) OR EXISTS(SELECT 1 FROM public.shared_schedule_executions link JOIN public.person_household_approvals approval ON approval.id=link.approval_id
  WHERE link.schedule_id=schedules.id AND approval.ended_at IS NULL))));
