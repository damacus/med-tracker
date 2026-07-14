# frozen_string_literal: true

class AllowInvitationTokenRlsBootstrap < ActiveRecord::Migration[8.1]
  def up
    execute <<~SQL
      CREATE OR REPLACE FUNCTION med_tracker.current_invitation_token_digest()
      RETURNS text
      LANGUAGE sql
      STABLE
      AS $$
        SELECT NULLIF(current_setting('med_tracker.current_invitation_token_digest', true), '');
      $$;
      DO $role_grant$
      BEGIN
        IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'med_tracker_app') THEN
          GRANT EXECUTE ON FUNCTION med_tracker.current_invitation_token_digest() TO med_tracker_app;
        END IF;
      END
      $role_grant$;

      DROP POLICY IF EXISTS household_tenant_isolation ON household_invitations;
      CREATE POLICY household_tenant_isolation ON household_invitations
      USING (
        household_id = med_tracker.current_household_id()
        OR (
          token_digest = med_tracker.current_invitation_token_digest()
          AND accepted_at IS NULL
          AND revoked_at IS NULL
          AND expires_at > CURRENT_TIMESTAMP
        )
      )
      WITH CHECK (household_id = med_tracker.current_household_id());
    SQL
  end

  def down
    execute <<~SQL
      DROP POLICY IF EXISTS household_tenant_isolation ON household_invitations;
      CREATE POLICY household_tenant_isolation ON household_invitations
      USING (household_id = med_tracker.current_household_id())
      WITH CHECK (household_id = med_tracker.current_household_id());
      DROP FUNCTION IF EXISTS med_tracker.current_invitation_token_digest();
    SQL
  end
end
