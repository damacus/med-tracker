# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'OTP Login', :browser do
  fixtures :accounts, :people, :users, :account_otp_keys

  it 'allows login with valid OTP code' do
    visit login_path

    fill_in 'Email address', with: 'damacus@example.com'
    fill_in 'Password', with: 'password'
    click_button 'Login'

    # Should redirect to OTP authentication page
    expect(page).to have_current_path('/otp-auth')
    expect(page).to have_content('Enter your authentication code')

    # Generate OTP code - TOTP codes are valid for 30 seconds
    # Rodauth typically allows a drift of ±30 seconds (previous/current/next code)
    totp = ROTP::TOTP.new('JBSWY3DPEHPK3PXP')
    otp_code = totp.now

    # Enter the generated OTP code
    fill_in 'otp-auth-code', with: otp_code
    click_button 'Authenticate'

    # Should successfully log in and redirect to dashboard or root (which redirects to dashboard)
    expect(page).to have_current_path('/').or have_current_path('/dashboard')
    expect(page).to have_content('Dashboard')
  end

  it 'allows login with OTP code within drift window' do
    visit login_path

    fill_in 'Email address', with: 'damacus@example.com'
    fill_in 'Password', with: 'password'
    click_button 'Login'

    expect(page).to have_current_path('/otp-auth')

    totp = ROTP::TOTP.new('JBSWY3DPEHPK3PXP')
    otp_code = totp.at(20.seconds.ago)

    fill_in 'otp-auth-code', with: otp_code
    click_button 'Authenticate'

    expect(page).to have_current_path('/').or have_current_path('/dashboard')
    expect(page).to have_content('Dashboard')
  end

  it 'shows error for invalid OTP code' do
    visit login_path

    fill_in 'Email address', with: 'damacus@example.com'
    fill_in 'Password', with: 'password'
    click_button 'Login'

    expect(page).to have_current_path('/otp-auth')

    # Enter invalid OTP code
    fill_in 'otp-auth-code', with: '000000'
    click_button 'Authenticate'

    # Should show error and stay on OTP page
    expect(page).to have_current_path('/otp-auth')
    expect(page).to have_content('Error logging in via TOTP authentication')
  end
end
