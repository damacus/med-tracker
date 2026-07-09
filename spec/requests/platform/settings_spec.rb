# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Platform settings' do
  fixtures :all

  let(:platform_user) { users(:admin) }
  let(:household_owner) { users(:damacus) }

  it 'allows an active platform admin outside household routes' do
    PlatformAdmin.create!(account: platform_user.person.account)
    sign_in(platform_user)

    get platform_settings_path

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Platform Settings')
  end

  it 'updates platform settings for an active platform admin' do
    PlatformAdmin.create!(account: platform_user.person.account)
    sign_in(platform_user)
    authenticate_platform_totp(platform_user.person.account)

    patch platform_settings_path, params: { app_settings: { invite_only: '0' } }

    expect(response).to redirect_to(platform_settings_path)
    expect(AppSettings.instance.reload.invite_only).to be(false)
  end

  it 'requires fresh privileged MFA before updating platform settings' do
    PlatformAdmin.create!(account: platform_user.person.account)
    sign_in(platform_user)

    patch platform_settings_path, params: { app_settings: { invite_only: '0' } }

    expect(response).to redirect_to(profile_path)
    expect(AppSettings.instance.reload.invite_only).not_to be(false)
  end

  it 'updates medicine lookup settings for an active platform admin' do
    PlatformAdmin.create!(account: platform_user.person.account)
    sign_in(platform_user)
    authenticate_platform_totp(platform_user.person.account)

    patch platform_settings_path,
          params: {
            app_settings: {
              medicine_lookup_base_url: 'https://terminology.example.test/fhir',
              medicine_lookup_token_url: 'https://auth.example.test/token',
              medicine_lookup_source_priority: %w[open_products_facts local_nhs_dmd curated_catalog]
            }
          }

    settings = AppSettings.instance.reload
    expect(response).to redirect_to(platform_settings_path)
    expect(settings.medicine_lookup_base_url).to eq('https://terminology.example.test/fhir')
    expect(settings.lookup_source_priority_for(%w[local_nhs_dmd open_products_facts])).to eq(
      %w[open_products_facts local_nhs_dmd]
    )
  end

  it 'renders validation errors when platform settings cannot be updated' do
    PlatformAdmin.create!(account: platform_user.person.account)
    settings = AppSettings.instance
    allow(AppSettings).to receive(:instance).and_return(settings)
    allow(settings).to receive(:update).and_return(false)
    sign_in(platform_user)
    authenticate_platform_totp(platform_user.person.account)

    patch platform_settings_path, params: { app_settings: { invite_only: '0' } }

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.body).to include('Platform Settings')
  end

  it 'denies a household owner without platform admin access' do
    sign_in(household_owner)

    get platform_settings_path

    expect(response).to redirect_to(root_path)
  end

  it 'denies lookup setting updates from household owners without platform admin access' do
    sign_in(household_owner)

    patch platform_settings_path,
          params: {
            app_settings: {
              medicine_lookup_base_url: 'https://terminology.example.test/fhir',
              medicine_lookup_source_priority: %w[open_products_facts local_nhs_dmd]
            }
          }

    expect(response).to redirect_to(root_path)
    expect(AppSettings.instance.reload.medicine_lookup_base_url).to eq(NhsDmd::Client::BASE_URL)
  end

  it 'keeps household admin settings denied to household managers without platform admin access' do
    sign_in(household_owner)

    get admin_settings_path

    expect(response).to redirect_to(root_path)
  end

  def authenticate_platform_totp(account)
    secret = 'jbswy3dpehpk3pxp'
    visible_secret = RodauthApp.rodauth.allocate.send(:otp_hmac_secret, secret)
    AccountOtpKey.where(id: account.id).delete_all
    AccountOtpKey.create!(id: account.id, key: secret, last_use: 5.minutes.ago)
    post '/otp-auth', params: { otp: ROTP::TOTP.new(visible_secret).at(Time.current) }
  end
end
