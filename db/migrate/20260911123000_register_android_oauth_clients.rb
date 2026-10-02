class RegisterAndroidOauthClients < ActiveRecord::Migration[8.1]
  def up
    %w[io.damacus.medtracker io.damacus.medtracker.debug io.damacus.medtracker.staging].each do |identifier|
      execute <<~SQL
        INSERT INTO oauth_applications
          (name, client_id, redirect_uri, scopes, client_kind, token_endpoint_auth_method, created_at, updated_at)
        VALUES ('MedTracker Android', #{connection.quote(identifier)},
                #{connection.quote("#{identifier}:/oauth2redirect")}, 'medtracker offline_access', 'mobile', 'none',
                CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        ON CONFLICT (client_id) DO NOTHING
      SQL
    end
  end

  def down
    raise ActiveRecord::IrreversibleMigration, 'Registered applications may have active account sessions'
  end
end
