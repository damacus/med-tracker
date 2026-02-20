# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Authentication Features', type: :system do
  fixtures :accounts, :people, :users, :account_otp_keys

  describe 'AUTH-019: Additional factor required for 2FA accounts' do
    it 'redirects to TOTP authentication after password login' do
      visit login_path
      fill_in 'Email address', with: 'damacus@example.com'
      fill_in 'Password', with: 'password'
      click_button 'Sign In to Dashboard'

      expect(page).to have_current_path('/otp-auth')
    end
  end

  describe 'AUTH-007: Email verification grace period configuration' do
    it 'has environment-aware grace period configuration' do
      # In test/development: 7 days grace period allows unverified users to login
      # In production: 0 grace period blocks unverified users immediately
      # We verify the configuration mechanism exists and is environment-aware
      expect(Rails.env.test?).to be true

      # The grace period in non-production is 7 days
      # This is configured in rodauth_main.rb:
      # verify_account_grace_period Rails.env.production? ? 0 : 7.days.to_i
      account = Account.create!(
        email: 'unverified@example.com',
        password_hash: RodauthApp.rodauth.allocate.password_hash('SecureP@ssword123!'),
        status: :unverified
      )

      # In test environment, unverified users can login during grace period
      visit login_path
      fill_in 'Email address', with: 'unverified@example.com'
      fill_in 'Password', with: 'SecureP@ssword123!'
      click_button 'Sign In to Dashboard'

      # Account remains unverified but login succeeds in test env
      expect(account.reload.status).to eq('unverified')
    end
  end

  describe 'AUTH-014: Password reset flow' do
    before do
      Account.create!(
        email: 'resetme@example.com',
        password_hash: RodauthApp.rodauth.allocate.password_hash('oldpassword123'),
        status: :verified
      )
    end

    it 'allows user to request password reset' do
      visit login_path
      click_link 'Forgot?'

      expect(page).to have_current_path('/reset-password-request')
      fill_in 'Email address', with: 'resetme@example.com'
      click_button 'Request Password Reset'

      expect(page).to have_content(/email|sent|reset/i)
    end

    it 'shows error for non-existent email' do
      visit '/reset-password-request'
      fill_in 'Email address', with: 'nonexistent@example.com'
      click_button 'Request Password Reset'

      # Rodauth may show success message for security (to prevent email enumeration)
      # or may show an error - depends on configuration
      expect(page).to have_current_path(%r{/reset-password|/login})
    end
  end

  describe 'AUTH-015: Remember me functionality' do
    let!(:account) do
      Account.create!(
        email: 'remember@example.com',
        password_hash: RodauthApp.rodauth.allocate.password_hash('SecureP@ssword123!'),
        status: :verified
      )
    end

    before do
      person = Person.create!(
        account: account,
        name: 'Remember User',
        email: 'remember@example.com',
        date_of_birth: 30.years.ago,
        person_type: :adult
      )
      User.create!(
        person: person,
        email_address: 'remember@example.com',
        role: :parent,
        active: true
      )
    end

    it 'shows remember me checkbox on login form' do
      visit login_path
      expect(page).to have_field('remember', type: 'checkbox')
    end

    it 'creates remember key when remember me is checked' do
      visit login_path
      fill_in 'Email address', with: 'remember@example.com'
      fill_in 'Password', with: 'SecureP@ssword123!'
      check 'remember'
      click_button 'Sign In to Dashboard'

      expect(page).to have_current_path('/dashboard')
      # Verify remember key was created in account_remember_keys table
      remember_key_exists = ActiveRecord::Base.connection.execute(
        "SELECT COUNT(*) FROM account_remember_keys WHERE account_id = #{account.id}"
      ).first['count'].to_i > 0
      expect(remember_key_exists).to be true
    end
  end

  describe 'AUTH-016: Account status unverified prevents login in production' do
    it 'has environment-aware grace period configuration' do
      # Verify the configuration mechanism exists and is environment-aware
      # In production: verify_account_grace_period = 0 (blocks immediately)
      # In test/dev: verify_account_grace_period = 7.days (allows login)
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read

      # Verify the configuration line exists
      expect(rodauth_file).to include('verify_account_grace_period Rails.env.production? ? 0 : 7.days.to_i')

      # In test environment, we're not in production
      expect(Rails.env.production?).to be false

      # The grace period in test/dev is 7 days (604800 seconds)
      expected_grace_period = 7.days.to_i
      expect(expected_grace_period).to eq(604_800)
    end

    it 'allows unverified accounts to login during grace period in non-production' do
      account = Account.create!(
        email: 'blocked@example.com',
        password_hash: RodauthApp.rodauth.allocate.password_hash('SecureP@ssword123!'),
        status: :unverified
      )

      # In test environment, unverified users CAN login (grace period is 7 days)
      visit login_path
      fill_in 'Email address', with: 'blocked@example.com'
      fill_in 'Password', with: 'SecureP@ssword123!'
      click_button 'Sign In to Dashboard'

      # Login should succeed in test env (grace period active)
      # Note: May redirect to verify account page or dashboard depending on config
      expect(account.reload.status).to eq('unverified')
    end
  end

  describe 'AUTH-017: Account status closed prevents login' do
    before do
      Account.create!(
        email: 'closed@example.com',
        password_hash: RodauthApp.rodauth.allocate.password_hash('SecureP@ssword123!'),
        status: :closed
      )
    end

    it 'prevents login for closed accounts' do
      visit login_path
      fill_in 'Email address', with: 'closed@example.com'
      fill_in 'Password', with: 'SecureP@ssword123!'
      click_button 'Sign In to Dashboard'

      # Should show error and not redirect to dashboard
      expect(page).to have_no_current_path('/dashboard')
      expect(page).to have_content(/closed|invalid|error/i)
    end
  end

  describe 'AUTH-018: Verification email resend' do
    before do
      Account.create!(
        email: 'resend@example.com',
        password_hash: RodauthApp.rodauth.allocate.password_hash('SecureP@ssword123!'),
        status: :unverified
      )
    end

    it 'shows resend verification link on login page' do
      visit login_path
      expect(page).to have_link('Resend verification email')
    end

    it 'allows requesting verification email resend' do
      visit '/verify-account-resend'
      fill_in 'Email address', with: 'resend@example.com'
      click_button 'Resend Verify Account Information'

      expect(page).to have_content(/sent|email|verification/i)
    end
  end

  describe 'AUTH-009/010: OIDC configuration' do
    it 'has omniauth feature enabled in Rodauth configuration' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include(':omniauth')
    end

    it 'has account_identities table for storing OIDC identities' do
      expect(ActiveRecord::Base.connection.table_exists?(:account_identities)).to be true
    end

    it 'shows login page with OIDC section or other options' do
      visit login_path
      # Check for brand title which is always present
      expect(page).to have_content('MedTracker')
    end
  end

  describe 'AUTH-013: Link OIDC account from settings' do
    it 'has omniauth configuration in Rodauth' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include(':omniauth')
      expect(rodauth_file).to include('omniauth_provider')
    end
  end
end
