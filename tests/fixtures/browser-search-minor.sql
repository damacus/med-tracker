UPDATE public.people SET person_type=1,has_capacity=false,date_of_birth=(current_date-interval '10 years')::date
WHERE id=73001
  AND id=(SELECT person_id FROM public.accounts WHERE id=71001)
  AND id=(SELECT person_id FROM public.household_memberships WHERE id=74001 AND household_id=72001);
