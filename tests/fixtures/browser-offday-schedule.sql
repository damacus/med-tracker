UPDATE public.schedules
SET start_date = current_date - 1,
    end_date = current_date + 1,
    schedule_type = 2,
    dose_amount = 2,
    schedule_config = jsonb_build_object('weekdays', jsonb_build_array(lower(to_char(current_date - 1, 'FMDay'))))
WHERE id = 83001 AND household_id = 72001;
