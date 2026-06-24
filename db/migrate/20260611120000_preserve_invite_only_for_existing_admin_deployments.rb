# frozen_string_literal: true

class PreserveInviteOnlyForExistingAdminDeployments < ActiveRecord::Migration[8.1]
  def up
    return if ENV.key?('INVITE_ONLY')
    return unless table_exists?(:app_settings)
    return unless table_exists?(:household_memberships)
    return unless existing_household_owners?

    if app_settings_empty?
      create_default_invite_only_settings
    else
      lock_unchanged_default_open_settings
    end
  end

  def down
    # Deliberately irreversible: this migration preserves the secure legacy
    # invite-only posture for existing household owner deployments.
  end

  private

  def existing_household_owners?
    select_value(<<~SQL.squish).present?
      SELECT 1 FROM household_memberships
      WHERE role = 'owner' AND status = 'active'
      LIMIT 1
    SQL
  end

  def app_settings_empty?
    select_value('SELECT 1 FROM app_settings LIMIT 1').blank?
  end

  def create_default_invite_only_settings
    now = quote(Time.current)
    execute <<~SQL.squish
      INSERT INTO app_settings (invite_only, created_at, updated_at)
      VALUES (TRUE, #{now}, #{now})
    SQL
  end

  def lock_unchanged_default_open_settings
    execute <<~SQL.squish
      UPDATE app_settings
      SET invite_only = TRUE, updated_at = CURRENT_TIMESTAMP
      WHERE invite_only = FALSE
        AND created_at = updated_at
    SQL
  end
end
