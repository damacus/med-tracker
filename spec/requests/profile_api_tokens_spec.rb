# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Profile API tokens' do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:user) { users(:damacus) }
  let(:account) { user.person.account }

  before do
    sign_in(user)
  end

  it 'creates a hashed app token from an MFA-satisfied web session and shows it only once' do
    configure_totp_for(account)
    allow(ApiAuthState).to receive(:web_session_mfa_satisfied?).and_return(true)
    membership = account.household_memberships.active.sole

    expect do
      post profile_api_tokens_path,
           params: { api_app_token: { name: 'CI deploy', household_membership_id: membership.id } }
    end.to change(ApiAppToken, :count).by(1)

    expect(response).to have_http_status(:created)
    raw_token = response.body.match(/mt_app_[A-Za-z0-9_-]+/).to_s
    app_token = ApiAppToken.order(:id).last

    expect(raw_token).to be_present
    expect(app_token).to have_attributes(
      account: account,
      household_membership: membership,
      name: 'CI deploy',
      revoked_at: nil
    )
    expect(app_token.token_digest).to eq(ApiAppToken.digest(raw_token))
    expect(response.body).to include('CI deploy')

    get profile_path

    expect(response.body).to include('CI deploy')
    expect(response.body).not_to include(raw_token)
  end

  it 'uses an app token as a bearer token for protected API resources' do
    configure_totp_for(account)
    allow(ApiAuthState).to receive(:web_session_mfa_satisfied?).and_return(true)
    membership = account.household_memberships.active.sole
    raw_token = create_app_token_from_profile(membership: membership)

    get api_v1_household_me_path(membership.household_id), headers: api_auth_headers(raw_token), as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'email_address')).to eq(user.email_address)
    expect(ApiAppToken.order(:id).last.reload.last_used_at).to be_within(5.seconds).of(Time.current)
  end

  it 'rejects app token creation when an MFA-configured account has not satisfied MFA in the web session' do
    configure_totp_for(account)
    membership = account.household_memberships.active.sole

    expect do
      post profile_api_tokens_path,
           params: { api_app_token: { name: 'CI deploy', household_membership_id: membership.id } }
    end.not_to change(ApiAppToken, :count)

    expect(response).to redirect_to(profile_path)
    expect(flash[:alert]).to include('two-factor authentication')
  end

  it 'revokes an app token and prevents later bearer use' do
    configure_totp_for(account)
    allow(ApiAuthState).to receive(:web_session_mfa_satisfied?).and_return(true)
    membership = account.household_memberships.active.sole
    raw_token = create_app_token_from_profile(membership: membership)
    app_token = ApiAppToken.order(:id).last

    delete profile_api_token_path(app_token)

    expect(response).to redirect_to(profile_path)
    expect(app_token.reload.revoked_at).to be_present

    get api_v1_household_me_path(membership.household_id), headers: api_auth_headers(raw_token), as: :json

    expect(response).to have_http_status(:unauthorized)
  end

  it 'rejects app tokens while the account is locked' do
    configure_totp_for(account)
    allow(ApiAuthState).to receive(:web_session_mfa_satisfied?).and_return(true)
    membership = account.household_memberships.active.sole
    raw_token = create_app_token_from_profile(membership: membership)
    lock_account!(account)

    get api_v1_household_me_path(membership.household_id), headers: api_auth_headers(raw_token), as: :json

    expect(response).to have_http_status(:unauthorized)
  end

  it 'rejects app tokens for inactive memberships' do
    configure_totp_for(account)
    allow(ApiAuthState).to receive(:web_session_mfa_satisfied?).and_return(true)
    membership = account.household_memberships.active.sole
    raw_token = create_app_token_from_profile(membership: membership)
    add_backup_owner_for(membership.household)
    membership.update!(status: :revoked)

    get api_v1_household_me_path(membership.household_id), headers: api_auth_headers(raw_token), as: :json

    expect(response).to have_http_status(:unauthorized)
  end

  def configure_totp_for(account)
    AccountOtpKey.create!(id: account.id, key: 'test_otp_key_secret')
  end

  def create_app_token_from_profile(membership:)
    post profile_api_tokens_path,
         params: { api_app_token: { name: 'CI deploy', household_membership_id: membership.id } }

    response.body.match(/mt_app_[A-Za-z0-9_-]+/).to_s
  end

  def lock_account!(account)
    AccountLockout.create!(
      account_id: account.id,
      key: SecureRandom.hex(16),
      deadline: 30.minutes.from_now
    )
  end

  def add_backup_owner_for(household)
    owner_account = Account.create!(
      email: "backup-owner-#{SecureRandom.hex(8)}@example.test",
      status: :verified
    )
    household.household_memberships.create!(account: owner_account, role: :owner, status: :active)
  end
end
