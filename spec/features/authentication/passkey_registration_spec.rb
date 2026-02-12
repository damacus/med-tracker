# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'PASSKEY-002: Passkey registration', browser: false, type: :system do
  fixtures :all

  before do
    driven_by(:rack_test)
  end

  let(:account) { accounts(:damacus) }

  describe 'Passkey management section on profile page' do
    before do
      login_as(account)
    end

    scenario 'User with no passkeys sees empty state' do
      visit profile_path

      expect(page).to have_content('Passkeys')
      expect(page).to have_content('No passkeys registered')
    end

    scenario 'User with passkeys sees list of registered passkeys' do
      account.account_webauthn_keys.create!(
        webauthn_id: 'test-credential-id',
        public_key: 'test-public-key',
        sign_count: 0,
        nickname: 'My MacBook'
      )

      visit profile_path

      expect(page).to have_content('My MacBook')
    end

    scenario 'User can navigate to add passkey page' do
      visit profile_path

      expect(page).to have_link('Add a passkey', href: '/webauthn-setup')
    end

    scenario 'User sees remove button for registered passkeys' do
      account.account_webauthn_keys.create!(
        webauthn_id: 'test-credential-id',
        public_key: 'test-public-key',
        sign_count: 0,
        nickname: 'My MacBook'
      )

      visit profile_path

      expect(page).to have_button('Remove')
    end
  end
end
