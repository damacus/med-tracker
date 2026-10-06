# frozen_string_literal: true

require 'rails_helper'

load Rails.root.join('db/migrate/20260911123000_register_android_oauth_clients.rb') unless
  defined?(RegisterAndroidOauthClients)

RSpec.describe RegisterAndroidOauthClients do
  it 'registers exact callbacks as public account clients without changing integrations' do
    described_class.new.up

    %w[io.damacus.medtracker io.damacus.medtracker.debug io.damacus.medtracker.staging].each do |identifier|
      client = OauthApplication.find_by!(client_id: identifier)
      expect(client).to have_attributes(client_kind: 'mobile', redirect_uri: "#{identifier}:/oauth2redirect",
                                        token_endpoint_auth_method: 'none', client_secret: nil, client_secret_hash: nil,
                                        scopes: 'medtracker offline_access')
    end
  end
end
