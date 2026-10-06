SELECT json_build_object(
  'accounts',(SELECT count(*) FROM public.accounts WHERE email=:'email'),
  'status',(SELECT status FROM public.accounts WHERE email=:'email'),
  'people',(SELECT count(*) FROM public.people p JOIN public.accounts a ON a.id=p.account_id WHERE a.email=:'email'),
  'users',(SELECT count(*) FROM public.users u JOIN public.people p ON p.id=u.person_id JOIN public.accounts a ON a.id=p.account_id WHERE a.email=:'email' AND u.active),
  'households',(SELECT count(*) FROM public.households h JOIN public.accounts a ON a.id=h.created_by_account_id WHERE a.email=:'email'),
  'owners',(SELECT count(*) FROM public.household_memberships m JOIN public.accounts a ON a.id=m.account_id WHERE a.email=:'email' AND m.role='owner' AND m.status='active' AND m.revoked_at IS NULL),
  'self_grants',(SELECT count(*) FROM public.person_access_grants g JOIN public.household_memberships m ON m.id=g.household_membership_id JOIN public.accounts a ON a.id=m.account_id WHERE a.email=:'email' AND g.person_id=m.person_id AND g.relationship_type='self' AND g.access_level='manage' AND g.revoked_at IS NULL),
  'household_name',(SELECT h.name FROM public.households h JOIN public.accounts a ON a.id=h.created_by_account_id WHERE a.email=:'email'),
  'household_slug',(SELECT h.slug FROM public.households h JOIN public.accounts a ON a.id=h.created_by_account_id WHERE a.email=:'email'));
