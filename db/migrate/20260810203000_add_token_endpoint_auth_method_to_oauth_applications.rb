# frozen_string_literal: true

class AddTokenEndpointAuthMethodToOauthApplications < ActiveRecord::Migration[8.1]
  CHECK_CONSTRAINT = 'chk_oauth_applications_token_auth_method'

  def up
    add_column :oauth_applications, :token_endpoint_auth_method, :string
    backfill_authentication_methods
    change_column_null :oauth_applications, :token_endpoint_auth_method, false
    add_check_constraint :oauth_applications, authentication_method_constraint, name: CHECK_CONSTRAINT
  end

  def down
    remove_check_constraint :oauth_applications, name: CHECK_CONSTRAINT
    remove_column :oauth_applications, :token_endpoint_auth_method
  end

  private

  def backfill_authentication_methods
    execute <<~SQL.squish
      UPDATE oauth_applications
      SET token_endpoint_auth_method = CASE
        WHEN NULLIF(client_secret, '') IS NULL AND NULLIF(client_secret_hash, '') IS NULL
          THEN 'none'
        ELSE 'client_secret_basic client_secret_post'
      END
    SQL
  end

  def authentication_method_constraint
    <<~SQL.squish
      (
        token_endpoint_auth_method = 'none'
        AND NULLIF(client_secret, '') IS NULL
        AND NULLIF(client_secret_hash, '') IS NULL
      ) OR (
        token_endpoint_auth_method IN (
          'client_secret_basic',
          'client_secret_post',
          'client_secret_basic client_secret_post'
        )
        AND (NULLIF(client_secret, '') IS NOT NULL OR NULLIF(client_secret_hash, '') IS NOT NULL)
      )
    SQL
  end
end
