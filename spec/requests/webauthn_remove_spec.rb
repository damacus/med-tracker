# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'WebAuthn removal' do
  fixtures :accounts, :people, :users

  let(:user) { users(:damacus) }
  let(:account) { user.person.account }
  let(:passkey) do
    account.account_webauthn_keys.create!(
      webauthn_id: 'existing-credential-id',
      public_key: 'existing-public-key',
      sign_count: 0,
      nickname: 'Existing Passkey'
    )
  end

  before do
    sign_in(user)
  end

  it 'removes the selected passkey through Rodauth built-in removal' do
    passkey
    secret = 'jbswy3dpehpk3pxp'
    visible_secret = RodauthApp.rodauth.allocate.send(:otp_hmac_secret, secret)
    AccountOtpKey.create!(id: account.id, key: secret, last_use: 5.minutes.ago)
    post '/otp-auth', params: { otp: ROTP::TOTP.new(visible_secret).at(Time.current) }
    expect(response).to have_http_status(:found)

    post '/webauthn-remove',
         params: {
           password: 'password',
           webauthn_remove: 'existing-credential-id'
         }

    expect(account.account_webauthn_keys.reload.count).to eq(0)
    expect(response).to redirect_to('/')
  end
end
