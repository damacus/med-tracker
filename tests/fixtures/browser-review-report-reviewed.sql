UPDATE public.medication_review_prompts
SET status = 'not_relevant', updated_at = clock_timestamp()
WHERE household_id = 72001 AND id BETWEEN 91001 AND 91048;
