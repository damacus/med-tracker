# frozen_string_literal: true

require 'rails_helper'
require 'webauthn/fake_client'

RSpec.describe 'WebAuthn setup' do
  fixtures :accounts, :people, :users

  let(:user) { users(:damacus) }
  let(:account) { user.person.account }

  before do
    sign_in(user)
    account.account_webauthn_user_ids.delete_all
  end

  it 'renders registration options and stores a WebAuthn user id for the account' do
    expect do
      get '/webauthn-setup'
    end.to change { account.account_webauthn_user_ids.reload.count }.from(0).to(1)

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('webauthn-setup-form')
    expect(response.body).to include('Register a Passkey')
    expect(response.body).to include('Passkey name')
    nickname_input = response.parsed_body.at_css('input[name="nickname"]')
    expect(nickname_input['placeholder']).to eq('e.g. iPhone, Work laptop, YubiKey')
    expect(response.body).not_to include('Unable to initialize passkey registration')
  end

  it 'registers a named passkey from a valid WebAuthn credential' do
    get '/webauthn-setup'

    document = response.parsed_body
    challenge = document.at_css('input[name="webauthn_setup_challenge"]')['value']
    challenge_hmac = document.at_css('input[name="webauthn_setup_challenge_hmac"]')['value']
    credential = WebAuthn::FakeClient.new(webauthn_origin).create(challenge: challenge, user_verified: true)

    expect do
      post '/webauthn-setup',
           params: {
             password: 'password',
             nickname: 'Work laptop',
             webauthn_setup: credential.to_json,
             webauthn_setup_challenge: challenge,
             webauthn_setup_challenge_hmac: challenge_hmac
           }
    end.to change { account.account_webauthn_keys.reload.count }.by(1)

    expect(account.account_webauthn_keys.last.nickname).to eq('Work laptop')
  end

  def webauthn_origin
    ENV.fetch('APP_URL', 'http://www.example.com')
  end
end
