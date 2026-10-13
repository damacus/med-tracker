UPDATE public.medication_review_edit_leases
SET expires_at = clock_timestamp() - interval '1 second'
WHERE household_id = 72001 AND prompt_id = 91001;
