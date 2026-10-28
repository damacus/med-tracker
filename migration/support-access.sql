ALTER TABLE public.support_access_sessions
  ADD COLUMN activated_at timestamp(6) without time zone,
  ADD COLUMN approved_at timestamp(6) without time zone,
  ADD COLUMN approved_by_account_id bigint,
  ADD COLUMN approved_by_membership_id bigint,
  ADD COLUMN approved_permissions_version integer;

CREATE POLICY people_support_read ON public.people FOR SELECT TO med_tracker_app
USING (
  EXISTS (
    SELECT 1
    FROM public.support_access_sessions s
    JOIN public.platform_admins pa ON pa.id = s.platform_admin_id
    JOIN public.accounts admin ON admin.id = pa.account_id
    JOIN public.households h ON h.id = s.household_id
    JOIN public.household_memberships approval
      ON approval.id = s.approved_by_membership_id
     AND approval.household_id = s.household_id
    JOIN public.accounts approver
      ON approver.id = s.approved_by_account_id
     AND approver.person_id = approval.person_id
    WHERE admin.id = med_tracker.current_account_id()
      AND admin.status = 2 AND admin.active AND pa.status = 'active'
      AND s.household_id = med_tracker.current_household_id()
      AND h.status = 'active' AND h.lifecycle_state = 'active'
      AND s.approved_at IS NOT NULL
      AND s.ended_at IS NULL AND s.expired_at IS NULL
      AND s.expires_at > pg_catalog.timezone('UTC',pg_catalog.clock_timestamp())
      AND approval.role = 'owner' AND approval.status = 'active'
      AND approval.revoked_at IS NULL
      AND approval.permissions_version = s.approved_permissions_version
      AND approver.status = 2 AND approver.active
      AND NOT EXISTS (SELECT 1 FROM public.account_lockouts l
                      WHERE l.account_id IN (admin.id,approver.id)
                        AND l.deadline > pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()))
      AND (
        (people.id = approver.person_id AND people.person_type = 0 AND people.has_capacity)
        OR (
          s.activated_at IS NOT NULL
          AND EXISTS (
            SELECT 1 FROM public.household_memberships subject
            WHERE subject.person_id = people.id AND subject.household_id = s.household_id
              AND subject.status = 'active' AND subject.revoked_at IS NULL
          )
        )
      )
  )
);
