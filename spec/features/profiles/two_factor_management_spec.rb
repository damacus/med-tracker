# frozen_string_literal: true

require 'rails_helper'
require 'rotp'

RSpec.describe 'Two-Factor Authentication Management', type: :system do
  fixtures :accounts, :people, :users

  let(:user) { users(:damacus) }
  let(:account) { user.person.account }

  before do
    login_as(user)
    visit profile_path
  end

  describe 'MFA setup flows' do
    before do
      AccountOtpKey.where(id: account.id).delete_all
      AccountRecoveryCode.where(id: account.id).delete_all
      account.account_webauthn_keys.destroy_all
    end

    context 'when a passkey is already registered' do
      before do
        account.account_webauthn_keys.create!(
          webauthn_id: 'existing-credential-id',
          public_key: 'existing-public-key',
          sign_count: 0,
          nickname: 'Existing Passkey'
        )
      end

      it 'redirects to the 2FA method chooser when starting TOTP setup' do
        visit profile_path
        click_link 'Set up authenticator app'

        expect(page).to have_current_path('/multifactor-manage')
      end

      it 'redirects to the 2FA method chooser when viewing recovery codes' do
        visit profile_path
        click_link 'Generate recovery codes'

        expect(page).to have_current_path('/multifactor-manage')
      end

      it 'redirects to the 2FA method chooser when adding another passkey' do
        visit profile_path
        click_link 'Add a passkey'

        expect(page).to have_current_path('/multifactor-manage')
      end

      it 'redirects to the 2FA method chooser when removing a passkey' do
        visit profile_path
        click_link 'Remove'

        expect(page).to have_current_path('/multifactor-manage')
        expect(account.account_webauthn_keys.count).to eq(1)
      end
    end

    it 'allows setting up and disabling TOTP' do
      visit '/otp-setup'

      secret = find("input[name='otp_secret']", visible: false).value
      totp = ROTP::TOTP.new(secret, issuer: 'MedTracker')

      fill_in 'Current Password', with: 'password'
      fill_in 'Authentication Code', with: totp.at(Time.current)
      click_button 'Enable Two-Factor Authentication'

      expect(AccountOtpKey.exists?(id: account.id)).to be true

      visit profile_path
      click_link 'Disable'
      expect(page).to have_current_path('/otp-disable')
      fill_in 'password', with: 'password'
      click_button 'Disable TOTP Authentication'

      expect(AccountOtpKey.exists?(id: account.id)).to be false
    end

    it 'allows generating recovery codes after 2FA setup' do
      setup_totp

      visit '/recovery-codes'
      fill_in 'Password', with: 'password'
      click_button 'View Authentication Recovery Codes'

      fill_in 'Password', with: 'password'
      click_button 'Add Authentication Recovery Codes'

      expect(page).to have_css('#recovery-codes')
      expect(AccountRecoveryCode.where(id: account.id).count).to be_positive
    end
  end

  def setup_totp
    visit '/otp-setup'

    secret = find("input[name='otp_secret']", visible: false).value
    totp = ROTP::TOTP.new(secret, issuer: 'MedTracker')

    fill_in 'Current Password', with: 'password'
    fill_in 'Authentication Code', with: totp.at(Time.current)
    click_button 'Enable Two-Factor Authentication'
  end

  describe 'disabling the authenticator app' do
    before do
      login_as(users(:damacus))
      AccountOtpKey.find_or_create_by!(id: account.id) do |key|
        key.key = 'test_otp_key_secret'
      end
      visit profile_path
    end

    it 'routes to the Rodauth OTP disable endpoint' do
      expect(page).to have_link('Disable', href: '/otp-disable')
    end
  end
end
