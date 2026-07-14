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

  it 'requires privileged MFA proof before opening support access' do
    post platform_support_access_sessions_path,
         params: {
           support_access_session: { household_id: target_household.id, reason: 'Investigate invitation issue' }
         }

    expect(response).to redirect_to(profile_path)
    expect(flash[:alert]).to include('Set up MFA or a passkey')
  end

  it 'rejects stale privileged MFA proof before opening support access' do
    travel_to 16.minutes.ago do
      authenticate_with_totp
    end

    expect do
      post platform_support_access_sessions_path,
           params: {
             support_access_session: { household_id: target_household.id, reason: 'Investigate invitation issue' }
           }
    end.not_to change(SupportAccessSession, :count)

    expect(response).to redirect_to('/multifactor-auth')
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

  it 'opens and audits support access through the forced-RLS application role' do
    authenticate_with_totp

    with_runtime_role do
      expect do
        post platform_support_access_sessions_path,
             params: {
               support_access_session: {
                 household_id: target_household.id,
                 reason: 'Investigate forced-RLS support access'
               }
             }
      end.to change(SupportAccessSession, :count).by(1)

      set_runtime_household
      expect(SecurityAuditEvent.where(
        household: target_household,
        event_type: 'support_access_session.started'
      ).count).to eq(1)
    end
  end

  it 'rejects support access for a held household' do
    authenticate_with_totp
    target_household.update!(lifecycle_state: :held)

    expect do
      post platform_support_access_sessions_path,
           params: {
             support_access_session: {
               household_id: target_household.id,
               reason: 'Attempt unavailable support access'
             }
           }
    end.not_to change(SupportAccessSession, :count)

    expect(response).to redirect_to(root_path)
    expect(flash[:alert]).to include('not authorized')
  end

  it 'requires hosted privileged MFA before support-mode admin access' do
    ENV['HOSTED_ADMIN_MFA_REQUIRED'] = 'true'
    SupportAccessSession.create!(
      platform_admin: platform_admin,
      household: target_household,
      reason: 'Investigate invitation delivery failure',
      mfa_verified_at: Time.current
    )

    get admin_root_path(household_slug: target_household.slug)

    expect(response).to redirect_to(profile_path(household_slug: target_household.slug))
    expect(flash[:alert]).to include('Set up MFA or a passkey')
  end

  it 'denies support-mode access at the exact expiry boundary' do
    timestamp = Time.current.change(usec: 0)
    SupportAccessSession.create!(
      platform_admin: platform_admin,
      household: target_household,
      reason: 'Investigate invitation delivery failure',
      mfa_verified_at: timestamp,
      starts_at: 30.minutes.ago,
      expires_at: timestamp
    )

    travel_to(timestamp) do
      get admin_root_path(household_slug: target_household.slug)
    end

    expect(response).to have_http_status(:redirect)
  end

  it 'does not use a support session for a different household' do
    authenticate_with_totp
    SupportAccessSession.create!(
      platform_admin: platform_admin,
      household: target_household,
      reason: 'Investigate invitation delivery failure',
      mfa_verified_at: Time.current
    )
    other_household = Household.create!(name: 'Other Supported Household', slug: 'other-supported-household')

    get admin_root_path(household_slug: other_household.slug)

    expect(response).to have_http_status(:redirect)
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

  it 'records explicit support access end only once' do
    authenticate_with_totp
    support_session = SupportAccessSession.create!(
      platform_admin: platform_admin,
      household: target_household,
      reason: 'Investigate invitation delivery failure',
      mfa_verified_at: Time.current
    )

    2.times { delete platform_support_access_session_path(support_session) }

    expect(SecurityAuditEvent.where(event_type: 'support_access_session.ended', household: target_household).count)
      .to eq(1)
  end

  it 'does not record explicit end after natural expiry was processed' do
    authenticate_with_totp
    support_session = SupportAccessSession.create!(
      platform_admin: platform_admin,
      household: target_household,
      reason: 'Investigate invitation delivery failure',
      mfa_verified_at: 1.hour.ago,
      starts_at: 1.hour.ago,
      expires_at: 30.minutes.ago
    )
    SupportAccessSessions::ExpiryProcessor.call

    expect do
      delete platform_support_access_session_path(support_session)
    end.not_to(
      change { SecurityAuditEvent.where(event_type: 'support_access_session.ended', household: target_household).count }
    )

    expect(support_session.reload).to have_attributes(ended_at: nil)
    expect(support_session.expired_at).to be_present
    expect(SecurityAuditEvent.where(event_type: 'support_access_session.expired', household: target_household).count)
      .to eq(1)
  end

  it 'records natural expiry instead of explicit end when an expired session is closed late' do
    authenticate_with_totp
    support_session = SupportAccessSession.create!(
      platform_admin: platform_admin,
      household: target_household,
      reason: 'Investigate invitation delivery failure',
      mfa_verified_at: 1.hour.ago,
      starts_at: 1.hour.ago,
      expires_at: 30.minutes.ago
    )

    delete platform_support_access_session_path(support_session)

    expect(support_session.reload).to have_attributes(ended_at: nil)
    expect(support_session.expired_at).to be_present
    expect(SecurityAuditEvent.where(event_type: 'support_access_session.expired', household: target_household).count)
      .to eq(1)
    expect(SecurityAuditEvent.where(event_type: 'support_access_session.ended', household: target_household))
      .to be_empty
  end

  def authenticate_with_totp
    secret = 'jbswy3dpehpk3pxp'
    visible_secret = RodauthApp.rodauth.allocate.send(:otp_hmac_secret, secret)
    AccountOtpKey.create!(id: account.id, key: secret, last_use: 5.minutes.ago)
    post '/otp-auth', params: { otp: ROTP::TOTP.new(visible_secret).at(Time.current) }
  end

  def with_runtime_role
    ActiveRecord::Base.connection.transaction(requires_new: true) do
      ActiveRecord::Base.connection.execute('SET LOCAL ROLE med_tracker_app')
      yield
      raise ActiveRecord::Rollback
    end
  end

  def set_runtime_household
    ActiveRecord::Base.connection.execute(
      "SELECT set_config('med_tracker.current_household_id', '#{target_household.id}', true)"
    )
  end
end
