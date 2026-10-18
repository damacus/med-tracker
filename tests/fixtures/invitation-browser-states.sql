DELETE FROM public.household_invitation_grants WHERE household_id = 72001;
DELETE FROM public.household_invitations WHERE household_id = 72001;
INSERT INTO public.household_invitations
  (id, household_id, email, membership_role, token_digest, invited_by_membership_id,
   expires_at, accepted_at, revoked_at, created_at, updated_at)
VALUES
  (87901, 72001, 'browser-pending@example.test', 'member', 'browser-status-pending', 74001,
   now() + interval '7 days', NULL, NULL, now(), now()),
  (87902, 72001, 'browser-expired@example.test', 'administrator', 'browser-status-expired', 74001,
   now() - interval '1 day', NULL, NULL, now() - interval '2 days', now()),
  (87903, 72001, 'browser-accepted@example.test', 'member', 'browser-status-accepted', 74001,
   now() + interval '7 days', now(), NULL, now() - interval '3 days', now()),
  (87904, 72001, 'browser-revoked@example.test', 'member', 'browser-status-revoked', 74001,
   now() + interval '7 days', NULL, now(), now() - interval '4 days', now());
