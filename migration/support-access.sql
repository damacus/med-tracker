ALTER TABLE public.support_access_sessions
  ADD COLUMN activated_at timestamp(6) without time zone,
  ADD COLUMN approved_at timestamp(6) without time zone,
  ADD COLUMN approved_by_account_id bigint,
  ADD COLUMN approved_by_membership_id bigint,
  ADD COLUMN approved_permissions_version integer;
