UPDATE public.household_invitations SET revoked_at=clock_timestamp() WHERE household_id=72001 AND email=:'email';
