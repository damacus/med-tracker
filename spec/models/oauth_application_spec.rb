# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OauthApplication do
  subject(:application) do
    described_class.new(
      name: 'SMART client',
      client_id: 'smart-client',
      redirect_uri: 'https://client.example/callback',
      scopes: 'launch/patient patient/*.rs offline_access'
    )
  end

  it 'accepts a registered HTTPS redirect URI and supported SMART scopes' do
    expect(application).to be_valid
  end

  it 'rejects redirect URIs that were not registered with HTTPS' do
    application.redirect_uri = 'http://client.example/callback'

    expect(application).not_to be_valid
  end

  it 'accepts multiple exact HTTPS redirect URIs' do
    application.redirect_uri = 'https://client.example/callback https://client.example/secondary-callback'

    expect(application).to be_valid
  end

  it 'rejects a registration when any redirect URI is not HTTPS' do
    application.redirect_uri = 'https://client.example/callback http://client.example/secondary-callback'

    expect(application).not_to be_valid
  end

  it 'rejects scopes outside the supported SMART contract' do
    application.scopes = 'patient/*.cruds'

    expect(application).not_to be_valid
  end
end
