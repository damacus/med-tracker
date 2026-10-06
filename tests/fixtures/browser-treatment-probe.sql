SELECT json_build_object(
  'schedules', (SELECT count(*) FROM public.schedules WHERE household_id=72001 AND person_id=73001 AND retired_at IS NULL),
  'schedule_active', (SELECT active FROM public.schedules WHERE household_id=72001 AND person_id=73001 AND retired_at IS NULL ORDER BY id DESC LIMIT 1),
  'schedule_amount', (SELECT dose_amount::text FROM public.schedules WHERE household_id=72001 AND person_id=73001 AND retired_at IS NULL ORDER BY id DESC LIMIT 1),
  'assignments', (SELECT count(*) FROM public.person_medications WHERE household_id=72001 AND person_id=73001 AND retired_at IS NULL),
  'open_pauses', (SELECT count(*) FROM public.medication_pause_periods WHERE household_id=72001 AND ended_at IS NULL),
  'schedule_audits', (SELECT count(*) FROM public.versions WHERE household_id=72001 AND item_type='Schedule'),
  'assignment_audits', (SELECT count(*) FROM public.versions WHERE household_id=72001 AND item_type='PersonMedication'),
  'supply', (SELECT current_supply::text FROM public.medications WHERE id=80001)
);
