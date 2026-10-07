DO $$
DECLARE
  schedule_id bigint;
BEGIN
  SELECT id INTO STRICT schedule_id FROM public.schedules WHERE household_id=72001 AND person_id=73001 AND retired_at IS NULL;
  IF (SELECT jsonb_array_length(schedule_config->'taper_steps') FROM public.schedules WHERE id=schedule_id) <> 2 THEN
    RAISE EXCEPTION 'Expected two owned taper steps';
  END IF;
  UPDATE public.schedules
  SET schedule_config=jsonb_set(schedule_config, '{taper_steps,1,min_hours_between_doses}', '"1.5"'::jsonb), updated_at=now()
  WHERE id=schedule_id;
END;
$$;
