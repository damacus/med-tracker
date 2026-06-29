# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Hosted admin MFA gate' do
  fixtures :all

  let(:admin) { users(:admin) }
  let(:regular_user) { users(:jane) }
  let(:account) { admin.person.account }

  around do |example|
    previous_value = ENV.fetch('HOSTED_ADMIN_MFA_REQUIRED', nil)
    ENV.delete('HOSTED_ADMIN_MFA_REQUIRED')
    example.run
  ensure
    if previous_value.nil?
      ENV.delete('HOSTED_ADMIN_MFA_REQUIRED')
    else
      ENV['HOSTED_ADMIN_MFA_REQUIRED'] = previous_value
    end
  end

  it 'does not change household admin access when hosted enforcement is disabled' do
    sign_in(admin)

    get admin_root_path

    expect(response).to have_http_status(:ok), response.location.to_s
  end

  it 'redirects hosted administrators without configured MFA to profile setup' do
    ENV['HOSTED_ADMIN_MFA_REQUIRED'] = 'true'
    sign_in(admin)

    get admin_root_path

    expect(response).to redirect_to(profile_path)
    expect(flash[:alert]).to include('Set up MFA or a passkey')
  end

  it 'denies hosted non-administrators without requiring privileged MFA setup' do
    ENV['HOSTED_ADMIN_MFA_REQUIRED'] = 'true'
    sign_in(regular_user)

    get admin_root_path

    expect(response).to redirect_to(root_path)
    expect(flash[:alert]).not_to include('Set up MFA or a passkey')
  end

  it 'redirects hosted administrators with unverified local MFA to Rodauth verification' do
    ENV['HOSTED_ADMIN_MFA_REQUIRED'] = 'true'
    sign_in(admin)
    AccountOtpKey.create!(id: account.id, key: 'test_otp_key_secret')

    get admin_root_path

    expect(response).to redirect_to('/multifactor-auth')
    expect(flash[:alert]).to include('Verify MFA or a passkey')
  end

  it 'allows hosted administrators after local MFA verification' do
    ENV['HOSTED_ADMIN_MFA_REQUIRED'] = 'true'
    sign_in(admin)
    secret = 'jbswy3dpehpk3pxp'
    visible_secret = RodauthApp.rodauth.allocate.send(:otp_hmac_secret, secret)
    AccountOtpKey.create!(id: account.id, key: secret, last_use: 5.minutes.ago)
    post '/otp-auth', params: { otp: ROTP::TOTP.new(visible_secret).at(Time.current) }

    get admin_root_path

    expect(response).to have_http_status(:ok)
  end

  it 'rejects stale local MFA session evidence when no local MFA method remains configured' do
    ENV['HOSTED_ADMIN_MFA_REQUIRED'] = 'true'
    sign_in(admin)
    allow(ApiAuthState).to receive(:web_session_mfa_method_present?).and_return(true)

    get admin_root_path

    expect(response).to redirect_to(profile_path)
  end

  it 'allows hosted administrators with upstream OIDC MFA proof' do
    ENV['HOSTED_ADMIN_MFA_REQUIRED'] = 'true'
    sign_in(admin)
    allow(ApiAuthState).to receive(:web_session_oidc_mfa_verified?).and_return(true)

    get admin_root_path

    expect(response).to have_http_status(:ok)
  end
end
