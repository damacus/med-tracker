client = OauthApplication.find_or_initialize_by(client_id: 'medtracker-ios-local')
client.assign_attributes(
  name: 'MedTracker iOS Local',
  client_kind: 'mobile',
  redirect_uri: 'io.damacus.medtracker.dev:/oauth2redirect',
  scopes: 'medtracker offline_access',
  token_endpoint_auth_method: 'none',
  client_secret: nil,
  client_secret_hash: nil
)
client.save!
