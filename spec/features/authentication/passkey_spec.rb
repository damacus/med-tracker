# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'PASSKEY-001: WebAuthn/Passkey configuration' do
  scenario 'WebAuthn gem is available' do
    expect { WebAuthn }.not_to raise_error
  end

  scenario 'WebAuthn database tables exist' do
    # Check that tables were created
    expect(ActiveRecord::Base.connection.table_exists?('account_webauthn_keys')).to be true
    expect(ActiveRecord::Base.connection.table_exists?('account_webauthn_user_ids')).to be true

    # Check indexes
    indexes = ActiveRecord::Base.connection.indexes('account_webauthn_keys')
    expect(indexes.map(&:name)).to include('index_account_webauthn_keys_on_account_id')
    expect(indexes.map(&:name)).to include('index_account_webauthn_keys_on_webauthn_id')
  end

  scenario 'WebAuthn models are defined' do
    expect { AccountWebauthnKey }.not_to raise_error
    expect { AccountWebauthnUserId }.not_to raise_error
    expect { WebAuthnService }.not_to raise_error
  end

  scenario 'Account model has WebAuthn associations' do
    account = create(:account)
    expect(account.respond_to?(:account_webauthn_keys)).to be true
    expect(account.respond_to?(:account_webauthn_user_ids)).to be true
  end

  scenario 'WebAuthnService can generate registration options' do
    account = create(:account)
    service = WebAuthnService.new(account)

    options = service.registration_options

    expect(options).to be_a(WebAuthn::PublicKeyCredential::CreationOptions)
    expect(options.rp.name).to eq('MedTracker')
    expect(options.user.id).to eq(account.id.to_s)
    expect(options.authenticator_selection[:user_verification]).to eq('required')
    expect(options.attestation).to eq('direct')
  end

  scenario 'WebAuthnService can generate authentication options' do
    account = create(:account)
    service = WebAuthnService.new(account)

    options = service.authentication_options

    expect(options).to be_a(WebAuthn::PublicKeyCredential::RequestOptions)
    expect(options.user_verification).to eq('required')
  end
end
