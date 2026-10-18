SELECT json_build_object(
  'accounts',(SELECT count(*) FROM public.accounts WHERE email=:'email'),
  'status',(SELECT status FROM public.accounts WHERE email=:'email'),
  'people',(SELECT count(*) FROM public.people WHERE id IN(SELECT person_id FROM public.accounts WHERE email=:'email') AND EXISTS(SELECT 1 FROM public.household_memberships m WHERE m.person_id=people.id AND m.household_id=72001)),
  'memberships',(SELECT count(*) FROM public.household_memberships WHERE person_id IN(SELECT person_id FROM public.accounts WHERE email=:'email') AND household_id=72001 AND status='active'),
  'accepted',(SELECT count(*) FROM public.household_invitations WHERE email=:'email' AND household_id=72001 AND accepted_at IS NOT NULL));
