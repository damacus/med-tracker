# frozen_string_literal: true

require 'rails_helper'

# rubocop:disable Capybara/NegationMatcherAfterVisit
RSpec.describe 'Two-Factor Soft Enforcement' do
  fixtures :accounts, :people, :users

  describe 'privileged users without 2FA' do
    it 'shows notice to administrator without blocking access' do
      user = users(:damacus) # administrator
      clear_2fa_for_account(user.person.account)

      login_as(user)
      visit dashboard_path

      expect(page).to have_content('For enhanced security, please set up two-factor authentication')
      expect(page).to have_current_path(dashboard_path)
    end

    it 'shows notice to doctor without blocking access' do
      user = users(:doctor) # doctor role
      clear_2fa_for_account(user.person.account)

      login_as(user)
      visit dashboard_path

      expect(page).to have_content('For enhanced security, please set up two-factor authentication')
      expect(page).to have_current_path(dashboard_path)
    end

    it 'shows notice to nurse without blocking access' do
      user = users(:nurse) # nurse role
      clear_2fa_for_account(user.person.account)

      login_as(user)
      visit dashboard_path

      expect(page).to have_content('For enhanced security, please set up two-factor authentication')
      expect(page).to have_current_path(dashboard_path)
    end
  end

  describe 'non-privileged users without 2FA' do
    it 'does not show notice to parent users' do
      user = users(:jane) # parent role
      clear_2fa_for_account(user.person.account)

      login_as(user)
      visit dashboard_path

      expect(page).to have_no_content('For enhanced security, please set up two-factor authentication')
    end

    it 'does not show notice to carer users' do
      user = users(:bob) # carer role
      clear_2fa_for_account(user.person.account)

      login_as(user)
      visit dashboard_path

      expect(page).to have_no_content('For enhanced security, please set up two-factor authentication')
    end
  end

  describe 'privileged users with 2FA enabled' do
    it 'does not show notice when administrator has TOTP enabled' do
      user = users(:damacus)
      login_as(user)

      # Set up TOTP after login
      AccountOtpKey.find_or_create_by!(id: user.person.account.id) do |key|
        key.key = 'JBSWY3DPEHPK3PXP'
      end

      visit dashboard_path

      expect(page).to have_no_content('For enhanced security, please set up two-factor authentication')
    end

    it 'does not show notice when administrator has passkey enabled' do
      user = users(:damacus)
      login_as(user)

      # Set up passkey after login
      user.person.account.account_webauthn_keys.create!(
        webauthn_id: 'test-id',
        public_key: 'test-key',
        sign_count: 0
      )

      visit dashboard_path

      expect(page).to have_no_content('For enhanced security, please set up two-factor authentication')
    end
  end

  describe 'notice suppression on 2FA setup pages' do
    it 'does not show notice on OTP setup page' do
      user = users(:damacus)
      clear_2fa_for_account(user.person.account)

      login_as(user)
      visit '/otp-setup'

      expect(page).to have_no_content('For enhanced security, please set up two-factor authentication')
    end

    it 'does not show notice on WebAuthn setup page' do
      user = users(:damacus)
      clear_2fa_for_account(user.person.account)

      login_as(user)
      visit '/webauthn-setup'

      expect(page).to have_no_content('For enhanced security, please set up two-factor authentication')
    end

    it 'does not show notice on recovery codes page' do
      user = users(:damacus)
      # Need at least one 2FA method for recovery codes
      AccountOtpKey.find_or_create_by!(id: user.person.account.id) do |key|
        key.key = 'JBSWY3DPEHPK3PXP'
      end

      login_as(user)
      visit '/recovery-codes'

      expect(page).to have_no_content('For enhanced security, please set up two-factor authentication')
    end
  end
end
# rubocop:enable Capybara/NegationMatcherAfterVisit
