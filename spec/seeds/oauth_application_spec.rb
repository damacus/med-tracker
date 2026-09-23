require 'rails_helper'

RSpec.describe OauthApplication do
  it 'registers a public native client and can be loaded repeatedly' do
    2.times { load Rails.root.join('db/seeds/seed_local_mobile_oauth_client.rb') }

    clients = described_class.where(client_id: 'medtracker-ios-local')
    expect(clients.count).to eq(1)
    expect(clients.first).to have_attributes(
      name: 'MedTracker iOS Local',
      client_kind: 'mobile',
      redirect_uri: 'io.damacus.medtracker.dev:/oauth2redirect',
      scopes: 'medtracker offline_access',
      token_endpoint_auth_method: 'none',
      client_secret: nil,
      client_secret_hash: nil
    )
  end
end
