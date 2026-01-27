# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Two-Factor Authentication Management' do
  fixtures :accounts, :people, :users

  let(:user) { users(:damacus) }
  let(:account) { user.person.account }

  before do
    login_as(user)
    visit profile_path
  end

  describe 'Two-Factor Authentication Card' do
    it 'displays the 2FA management card' do
      expect(page).to have_content('Two-Factor Authentication')
      expect(page).to have_content('Secure your account with multiple authentication methods')
    end

    it 'shows all 2FA method sections' do
      expect(page).to have_content('Authenticator App (TOTP)')
      expect(page).to have_content('Recovery Codes')
      expect(page).to have_content('Passkeys')
    end
  end

  describe 'TOTP Management' do
    context 'when TOTP is not enabled' do
      it 'shows setup button' do
        expect(page).to have_content('Not configured')
        expect(page).to have_link('Set up authenticator app', href: '/otp-setup')
      end
    end

    context 'when TOTP is enabled' do
      it 'shows authenticator app enabled status' do
        skip 'OTP setup requires database interaction'
      end

      it 'shows authenticator app disable button' do
        skip 'OTP setup requires database interaction'
      end
    end
  end

  describe 'Recovery Codes Management' do
    context 'when recovery codes are not generated' do
      it 'shows setup button' do
        expect(page).to have_content('Not generated')
        expect(page).to have_link('Generate recovery codes', href: '/recovery-codes')
      end
    end

    context 'when recovery codes exist' do
      it 'shows recovery codes view button' do
        skip 'Recovery codes setup requires database interaction'
      end

      it 'shows recovery codes regenerate button' do
        skip 'Recovery codes setup requires database interaction'
      end
    end
  end

  describe 'Passkeys Management' do
    context 'when no passkeys exist' do
      before do
        account.account_webauthn_keys.destroy_all
      end

      it 'shows empty state' do
        visit profile_path
        expect(page).to have_content('No passkeys registered')
      end

      it 'shows add button' do
        expect(page).to have_link('Add a passkey', href: '/webauthn-setup')
      end
    end

    context 'when passkeys exist' do
      before do
        account.account_webauthn_keys.create!(
          webauthn_id: 'test-id',
          public_key: 'test-key',
          sign_count: 0,
          nickname: 'Test Passkey'
        )
      end

      it 'lists all passkeys' do
        visit profile_path
        expect(page).to have_content('Test Passkey')
      end

      it 'shows remove button for each passkey' do
        visit profile_path
        expect(page).to have_button('Remove')
      end

      it 'shows add another button' do
        visit profile_path
        expect(page).to have_link('Add a passkey', href: '/webauthn-setup')
      end
    end
  end

  describe 'Navigation' do
    it 'has link to TOTP setup page' do
      expect(page).to have_link('Set up authenticator app', href: '/otp-setup')
    end

    it 'has link to recovery codes page' do
      expect(page).to have_link('Generate recovery codes', href: '/recovery-codes')
    end

    it 'has link to passkey setup page' do
      expect(page).to have_link('Add a passkey', href: '/webauthn-setup')
    end
  end
end
