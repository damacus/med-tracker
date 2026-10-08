UPDATE public.schedules
SET start_date = current_date - 1,
    end_date = current_date + 1,
    schedule_type = 5,
    dose_amount = 2,
    schedule_config = jsonb_build_object(
      'taper_steps', jsonb_build_array(
        jsonb_build_object('start_date', (current_date - 1)::text, 'end_date', (current_date - 1)::text, 'amount', '3', 'unit', 'tablet'),
        jsonb_build_object('start_date', current_date::text, 'end_date', current_date::text, 'amount', '1', 'unit', 'tablet'),
        jsonb_build_object('start_date', (current_date + 1)::text, 'end_date', (current_date + 1)::text, 'amount', '4', 'unit', 'tablet')
      )
    )
WHERE id = 83001 AND household_id = 72001;
