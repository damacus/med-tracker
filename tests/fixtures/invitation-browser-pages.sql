DELETE FROM public.household_invitation_grants WHERE household_id = 72001;
DELETE FROM public.household_invitations WHERE household_id = 72001;
INSERT INTO public.household_invitations
  (id, household_id, email, membership_role, token_digest, invited_by_membership_id,
   expires_at, created_at, updated_at)
SELECT 88000 + n, 72001, 'browser-page-' || n || '@example.test', 'member',
       'browser-page-digest-' || n, 74001, now() + interval '7 days',
       timestamp '2026-01-01 00:00:00', timestamp '2026-01-01 00:00:00'
FROM generate_series(1, 101) AS n;
