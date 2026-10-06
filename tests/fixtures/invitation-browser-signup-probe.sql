SELECT json_build_object(
  'accounts',(SELECT count(*) FROM public.accounts WHERE email=:'email'),
  'status',(SELECT status FROM public.accounts WHERE email=:'email'),
  'people',(SELECT count(*) FROM public.people WHERE account_id IN(SELECT id FROM public.accounts WHERE email=:'email') AND household_id=72001),
  'memberships',(SELECT count(*) FROM public.household_memberships WHERE account_id IN(SELECT id FROM public.accounts WHERE email=:'email') AND household_id=72001 AND status='active'),
  'accepted',(SELECT count(*) FROM public.household_invitations WHERE email=:'email' AND household_id=72001 AND accepted_at IS NOT NULL));
