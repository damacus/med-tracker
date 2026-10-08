DO $$
DECLARE
  option_id bigint;
  schedule_id bigint;
BEGIN
  SELECT id INTO STRICT option_id FROM public.dosages WHERE household_id=72001 AND medication_id=80001;
  SELECT id INTO STRICT schedule_id FROM public.schedules WHERE household_id=72001 AND person_id=73001 AND retired_at IS NULL;
  UPDATE public.dosages SET default_min_hours_between_doses=1.5, updated_at=now() WHERE id=option_id;
  UPDATE public.schedules
  SET schedule_config=jsonb_set(schedule_config, '{taper_steps,0,min_hours_between_doses}', '"1.5"'::jsonb), updated_at=now()
  WHERE id=schedule_id AND schedule_config #> '{taper_steps,0}' IS NOT NULL;
  IF NOT FOUND THEN
    RAISE EXCEPTION 'Expected an owned taper step';
  END IF;
END;
$$;
