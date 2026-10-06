SELECT json_build_object(
  'people',(SELECT count(*) FROM public.people WHERE account_id=71002 AND household_id=72001),
  'memberships',(SELECT count(*) FROM public.household_memberships WHERE account_id=71002 AND household_id=72001 AND status='active'),
  'self_grants',(SELECT count(*) FROM public.person_access_grants g JOIN public.household_memberships m ON m.id=g.household_membership_id WHERE m.account_id=71002 AND m.household_id=72001 AND g.person_id=m.person_id AND g.relationship_type='self' AND g.access_level='manage' AND g.revoked_at IS NULL),
  'dependent_grants',(SELECT count(*) FROM public.person_access_grants g JOIN public.household_memberships m ON m.id=g.household_membership_id WHERE m.account_id=71002 AND m.household_id=72001 AND g.person_id=73002 AND g.access_level='manage' AND g.revoked_at IS NULL),
  'relationships',(SELECT count(*) FROM public.carer_relationships r JOIN public.people p ON p.id=r.carer_id WHERE p.account_id=71002 AND r.household_id=72001 AND r.patient_id=73002 AND r.active),
  'source_household',(SELECT household_id FROM public.people WHERE id=73005),
  'accepted',(SELECT count(*) FROM public.household_invitations WHERE household_id=72001 AND email='foreign@example.test' AND accepted_at IS NOT NULL));
