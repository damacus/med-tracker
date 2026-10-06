SELECT json_build_object(
  'count',(SELECT count(*) FROM public.household_invitations WHERE household_id=72001 AND email=:'email'),
  'grants',(SELECT count(*) FROM public.household_invitation_grants WHERE household_id=72001 AND household_invitation_id IN(SELECT id FROM public.household_invitations WHERE household_id=72001 AND email=:'email')),
  'expired',(SELECT expires_at<clock_timestamp() FROM public.household_invitations WHERE household_id=72001 AND email=:'email' ORDER BY id DESC LIMIT 1));
