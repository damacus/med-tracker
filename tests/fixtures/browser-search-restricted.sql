UPDATE public.household_memberships SET role='member' WHERE id=74001;
UPDATE public.person_access_grants SET revoked_at=now() WHERE household_membership_id=74001;
