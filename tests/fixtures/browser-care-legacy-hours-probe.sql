SELECT json_build_object(
  'dose_option', (SELECT default_min_hours_between_doses::text FROM public.dosages WHERE household_id=72001 AND medication_id=80001),
  'taper', (SELECT schedule_config #>> '{taper_steps,0,min_hours_between_doses}' FROM public.schedules WHERE household_id=72001 AND person_id=73001 AND retired_at IS NULL)
);
