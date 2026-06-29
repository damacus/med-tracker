# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Platform support access sessions' do
  fixtures :all

  let(:platform_user) { users(:admin) }
  let(:account) { platform_user.person.account }
  let(:platform_admin) { PlatformAdmin.create!(account: account) }
  let(:target_household) { Household.create!(name: 'Supported Household', slug: 'supported-household') }

  before do
    platform_admin
    sign_in(platform_user)
  end

  it 'requires privileged MFA proof before opening support access' do
    post platform_support_access_sessions_path,
         params: {
           support_access_session: { household_id: target_household.id, reason: 'Investigate invitation issue' }
         }

    expect(response).to redirect_to(profile_path)
    expect(flash[:alert]).to include('Set up MFA or a passkey')
  end

  it 'requires a reason before opening support access' do
    authenticate_with_totp

    expect do
      post platform_support_access_sessions_path,
           params: { support_access_session: { household_id: target_household.id, reason: '' } }
    end.not_to change(SupportAccessSession, :count)

    expect(response).to redirect_to(platform_settings_path)
    expect(flash[:alert]).to include("Reason can't be blank")
  end

  it 'opens support access without granting household membership and records an audit event' do
    authenticate_with_totp
    initial_audit_count = SecurityAuditEvent.count
    proof_time = Time.current

    expect do
      post platform_support_access_sessions_path,
           params: {
             support_access_session: {
               household_id: target_household.id,
               reason: 'Investigate invitation delivery failure'
             }
           }
    end.to change(SupportAccessSession, :count).by(1)

    support_session = SupportAccessSession.order(:created_at).last
    audit_event = SecurityAuditEvent.order(:created_at).last

    expect(SecurityAuditEvent.count).to eq(initial_audit_count + 1)
    expect(response).to redirect_to(platform_settings_path)
    expect(support_session).to have_attributes(
      platform_admin: platform_admin,
      household: target_household,
      reason: 'Investigate invitation delivery failure',
      ended_at: nil
    )
    expect(support_session.mfa_verified_at).to be_within(2.seconds).of(proof_time)
    expect(support_session.request_id).to be_present
    expect(support_session.ip).to be_present
    expect(target_household.household_memberships.exists?(account: account)).to be(false)
    expect(audit_event).to have_attributes(
      household: target_household,
      actor_account: account,
      event_type: 'support_access_session.started'
    )
    expect(audit_event.metadata).to include(
      'support_access_session_id' => support_session.id
    )
    expect(audit_event.metadata).not_to include('reason')

    get admin_root_path(household_slug: target_household.slug)

    expect(response).to have_http_status(:ok)
  end

  it 'ends support access and records an audit event' do
    authenticate_with_totp
    support_session = SupportAccessSession.create!(
      platform_admin: platform_admin,
      household: target_household,
      reason: 'Investigate invitation delivery failure',
      mfa_verified_at: Time.current
    )

    expect do
      delete platform_support_access_session_path(support_session)
    end.to change(SecurityAuditEvent, :count).by(1)

    audit_event = SecurityAuditEvent.order(:created_at).last

    expect(response).to redirect_to(platform_settings_path)
    expect(support_session.reload.ended_at).to be_present
    expect(audit_event).to have_attributes(
      household: target_household,
      actor_account: account,
      event_type: 'support_access_session.ended'
    )
    expect(audit_event.metadata).to include('support_access_session_id' => support_session.id)
  end

  def authenticate_with_totp
    secret = 'jbswy3dpehpk3pxp'
    visible_secret = RodauthApp.rodauth.allocate.send(:otp_hmac_secret, secret)
    AccountOtpKey.create!(id: account.id, key: secret, last_use: 5.minutes.ago)
    post '/otp-auth', params: { otp: ROTP::TOTP.new(visible_secret).at(Time.current) }
  end
end
