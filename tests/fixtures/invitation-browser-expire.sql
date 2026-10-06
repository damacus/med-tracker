UPDATE public.household_invitations SET expires_at=clock_timestamp()-interval '1 minute'
WHERE household_id=72001 AND email=:'email';
