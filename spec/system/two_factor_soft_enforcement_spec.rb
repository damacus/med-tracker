# frozen_string_literal: true

require 'rails_helper'

# rubocop:disable Capybara/NegationMatcherAfterVisit
RSpec.describe 'Two-Factor Soft Enforcement', browser: false do
  fixtures :accounts, :people, :users

  before do
    driven_by(:rack_test)
  end

  describe 'privileged users without 2FA' do
    it 'shows notice to administrator on profile page' do
      user = users(:damacus) # administrator
      clear_2fa_for_account(user.person.account)

      login_as(user)
      visit profile_path

      expect(page).to have_content('For enhanced security, please set up two-factor authentication')
      expect(page).to have_current_path(profile_path)
    end

    it 'does not show notice to administrator on dashboard' do
      user = users(:damacus) # administrator
      clear_2fa_for_account(user.person.account)

      login_as(user)
      visit dashboard_path

      expect(page).to have_no_content('For enhanced security, please set up two-factor authentication')
    end

    it 'shows notice to doctor on profile page' do
      user = users(:doctor) # doctor role
      clear_2fa_for_account(user.person.account)

      login_as(user)
      visit profile_path

      expect(page).to have_content('For enhanced security, please set up two-factor authentication')
      expect(page).to have_current_path(profile_path)
    end

    it 'shows notice to nurse on profile page' do
      user = users(:nurse) # nurse role
      clear_2fa_for_account(user.person.account)

      login_as(user)
      visit profile_path

      expect(page).to have_content('For enhanced security, please set up two-factor authentication')
      expect(page).to have_current_path(profile_path)
    end
  end

  describe 'non-privileged users without 2FA' do
    it 'does not show notice to parent users on profile page' do
      user = users(:jane) # parent role
      clear_2fa_for_account(user.person.account)

      login_as(user)
      visit profile_path

      expect(page).to have_no_content('For enhanced security, please set up two-factor authentication')
    end

    it 'does not show notice to carer users on profile page' do
      user = users(:bob) # carer role
      clear_2fa_for_account(user.person.account)

      login_as(user)
      visit profile_path

      expect(page).to have_no_content('For enhanced security, please set up two-factor authentication')
    end
  end

  describe 'privileged users with 2FA enabled' do
    it 'does not show notice on profile page when administrator has TOTP enabled' do
      user = users(:damacus)
      login_as(user)

      # Set up TOTP after login
      AccountOtpKey.find_or_create_by!(id: user.person.account.id) do |key|
        key.key = 'JBSWY3DPEHPK3PXP'
      end

      visit profile_path

      expect(page).to have_no_content('For enhanced security, please set up two-factor authentication')
    end

    it 'does not show notice on profile page when administrator has passkey enabled' do
      user = users(:damacus)
      login_as(user)

      # Set up passkey after login
      user.person.account.account_webauthn_keys.create!(
        webauthn_id: 'test-id',
        public_key: 'test-key',
        sign_count: 0
      )

      visit profile_path

      expect(page).to have_no_content('For enhanced security, please set up two-factor authentication')
    end
  end
end
# rubocop:enable Capybara/NegationMatcherAfterVisit
