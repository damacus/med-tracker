CREATE POLICY personal_history_source ON public.schedules FOR SELECT TO med_tracker_owner
USING (current_setting('med_tracker.personal_history_access',true) = 'true'
  AND person_id = (SELECT person_id FROM public.accounts WHERE id = med_tracker.current_account_id() AND status = 2 AND active));

CREATE POLICY personal_history_source ON public.person_medications FOR SELECT TO med_tracker_owner
USING (current_setting('med_tracker.personal_history_access',true) = 'true'
  AND person_id = (SELECT person_id FROM public.accounts WHERE id = med_tracker.current_account_id() AND status = 2 AND active));

CREATE POLICY personal_history_take ON public.medication_takes FOR SELECT TO med_tracker_owner
USING (current_setting('med_tracker.personal_history_access',true) = 'true'
  AND (schedule_id IN (SELECT id FROM public.schedules WHERE person_id = (SELECT person_id FROM public.accounts WHERE id = med_tracker.current_account_id() AND status = 2 AND active))
    OR person_medication_id IN (SELECT id FROM public.person_medications WHERE person_id = (SELECT person_id FROM public.accounts WHERE id = med_tracker.current_account_id() AND status = 2 AND active))));

CREATE POLICY personal_history_medicine ON public.medications FOR SELECT TO med_tracker_owner
USING (current_setting('med_tracker.personal_history_access',true) = 'true'
  AND (id IN (SELECT taken_from_medication_id FROM public.medication_takes)
    OR id IN (SELECT medication_id FROM public.schedules)
    OR id IN (SELECT medication_id FROM public.person_medications)));
