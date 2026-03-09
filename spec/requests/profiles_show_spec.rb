# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Profiles' do
  fixtures :accounts, :people, :users

  let(:user) { users(:damacus) }
  let(:account) { user.person.account }

  before do
    sign_in(user)
  end

  describe 'GET /profile' do
    it 'renders the profile structure and account security content' do
      get profile_path

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('My Profile')
      expect(response.body).to include(user.name)
      expect(response.body).to include(account.email)
      expect(response.body).to include('Account Security')
      expect(response.body).to include('Change Email Address')
      expect(response.body).to include('Change Password')
      expect(response.body).to include('Danger Zone')
      expect(response.body).to include('Close Account')
      expect(response.body.scan('data-turbo-frame="modal"').size).to be >= 2
      expect(response.body).to include('ruby-ui--alert-dialog')
    end

    it 'renders the two-factor card and empty-state setup actions' do
      AccountOtpKey.where(id: account.id).delete_all
      AccountRecoveryCode.where(id: account.id).delete_all
      account.account_webauthn_keys.destroy_all

      get profile_path

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('Two-Factor Authentication')
      expect(response.body).to include('Authenticator App (TOTP)')
      expect(response.body).to include('Recovery Codes')
      expect(response.body).to include('Passkeys')
      expect(response.body).to include('Set up authenticator app')
      expect(response.body).to include('Generate recovery codes')
      expect(response.body).to include('No passkeys registered')
      expect(response.body).to include('Add a passkey')
    end

    it 'renders configured two-factor states without a browser round-trip' do
      AccountOtpKey.find_or_create_by!(id: account.id) do |key|
        key.key = 'test_otp_key_secret'
      end
      AccountRecoveryCode.where(id: account.id).delete_all
      5.times do |i|
        AccountRecoveryCode.create!(id: account.id, code: "recovery-code-#{i}")
      end
      account.account_webauthn_keys.destroy_all
      account.account_webauthn_keys.create!(
        webauthn_id: 'test-id',
        public_key: 'test-key',
        sign_count: 0,
        nickname: 'Test Passkey'
      )

      get profile_path

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('Authenticator app is active')
      expect(response.body).to include('Disable')
      expect(response.body).to include('Recovery codes generated')
      expect(response.body).to include('View codes')
      expect(response.body).to include('Regenerate')
      expect(response.body).to include('Test Passkey')
      expect(response.body).to include('Remove')
      expect(response.body).to include('Add a passkey')
    end
  end
end
