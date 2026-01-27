# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Two-Factor Authentication Management', type: :system do
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
      before do
        AccountOtpKey.where(id: account.id).delete_all
        visit profile_path
      end

      it 'shows setup button' do
        expect(page).to have_content('Not configured')
        expect(page).to have_link('Set up authenticator app', href: '/otp-setup')
      end
    end

    context 'when TOTP is enabled' do
      before do
        AccountOtpKey.find_or_create_by!(id: account.id) do |key|
          key.key = 'test_otp_key_secret'
        end
        visit profile_path
      end

      it 'shows authenticator app enabled status' do
        expect(page).to have_content('Authenticator app is active')
      end

      it 'shows authenticator app disable button' do
        expect(page).to have_link('Disable', href: '/otp-disable')
      end
    end
  end

  describe 'Recovery Codes Management' do
    context 'when recovery codes are not generated' do
      before do
        AccountRecoveryCode.where(id: account.id).delete_all
        visit profile_path
      end

      it 'shows setup button' do
        expect(page).to have_content('Not generated')
        expect(page).to have_link('Generate recovery codes', href: '/recovery-codes')
      end
    end

    context 'when recovery codes exist' do
      before do
        ActiveRecord::Base.connection.execute(
          "DELETE FROM account_recovery_codes WHERE id = #{account.id}"
        )
        5.times do |i|
          ActiveRecord::Base.connection.execute(
            "INSERT INTO account_recovery_codes (id, code) VALUES (#{account.id}, 'recovery-code-#{i}')"
          )
        end
        visit profile_path
      end

      it 'shows recovery codes view button' do
        expect(page).to have_link('View codes', href: '/recovery-codes')
      end

      it 'shows recovery codes regenerate button' do
        expect(page).to have_button('Regenerate')
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
    before do
      AccountOtpKey.where(id: account.id).delete_all
      visit profile_path
    end

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
