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

    expect do
      post '/webauthn-remove',
           params: {
             password: 'password',
             webauthn_remove: 'existing-credential-id'
           }
    end.to change { account.account_webauthn_keys.reload.count }.by(-1)

    expect(response).to redirect_to('/')
  end
end
