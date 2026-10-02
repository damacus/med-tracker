class AddMobileAccountOauthGrants < ActiveRecord::Migration[8.1]
  def change
    add_column :oauth_applications, :client_kind, :string, null: false, default: 'integration'
    add_check_constraint :oauth_applications, "client_kind IN ('integration', 'mobile')", name: 'oauth_client_kind'
    add_index :oauth_applications, %i[id client_kind], unique: true

    add_column :oauth_grants, :client_kind, :string, null: false, default: 'integration'
    add_column :oauth_grants, :authenticated_at, :datetime
    add_column :oauth_grants, :device_name, :string
    change_column_null :oauth_grants, :household_membership_id, true
    change_column_null :oauth_grants, :person_id, true
    change_column_null :oauth_grants, :permissions_version, true
    add_check_constraint :oauth_grants, <<~SQL.squish, name: 'oauth_grant_authority_boundary'
      (client_kind = 'integration' AND household_membership_id IS NOT NULL
       AND person_id IS NOT NULL AND permissions_version IS NOT NULL)
      OR (client_kind = 'mobile' AND household_membership_id IS NULL
          AND person_id IS NULL AND permissions_version IS NULL
          AND authenticated_at IS NOT NULL AND last_used_at IS NOT NULL)
    SQL
    add_foreign_key :oauth_grants, :oauth_applications,
                   column: %i[oauth_application_id client_kind], primary_key: %i[id client_kind],
                   name: 'oauth_grant_application_kind'
  end
end
