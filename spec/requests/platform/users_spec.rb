# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Platform users' do
  fixtures :all

  let(:platform_user) { users(:admin) }
  let(:household_owner) { users(:damacus) }
  let(:target_user) { users(:jane) }

  before do
    ensure_platform_admin!(platform_user.person.account)
  end

  it 'shows household and system administration roles separately' do
    sign_in(platform_user)

    get platform_users_path

    expect(response).to have_http_status(:ok)
    expect(response.body).to include(target_user.email_address)
    expect(response.body).to include('Household role')
    expect(response.body).to include('System access')
  end

  it 'shows owner promotion controls for non-owner household memberships' do
    ensure_household_membership!(target_user.person.account, target_user.person, role: :member)
    sign_in(platform_user)

    get platform_users_path

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Promote to owner')
  end

  it 'elevates a household user to system administrator' do
    sign_in(platform_user)
    authenticate_platform_totp(platform_user.person.account)

    expect do
      patch platform_user_path(target_user), params: { platform_user: { system_administrator: '1' } }
    end.to change(PlatformAdmin.active, :count).by(1)

    expect(response).to redirect_to(platform_users_path)
    expect(target_user.person.account.platform_admin.reload).to be_active
  end

  it 'removes system administrator access without changing household ownership' do
    ensure_platform_admin!(target_user.person.account)
    target_membership = ensure_household_membership!(target_user.person.account, target_user.person, role: :owner)
    sign_in(platform_user)
    authenticate_platform_totp(platform_user.person.account)

    patch platform_user_path(target_user), params: { platform_user: { system_administrator: '0' } }

    expect(response).to redirect_to(platform_users_path)
    expect(target_user.person.account.platform_admin.reload).to be_disabled
    expect(target_membership.reload).to be_owner
  end

  it 'requires fresh privileged MFA before changing system administrator access' do
    sign_in(platform_user)

    expect do
      patch platform_user_path(target_user), params: { platform_user: { system_administrator: '1' } }
    end.not_to change(PlatformAdmin.active, :count)

    expect(response).to redirect_to(profile_path)
    expect(target_user.person.account.platform_admin).to be_nil
  end

  it 'denies household owners without system administrator access' do
    sign_in(household_owner)

    patch platform_user_path(target_user), params: { platform_user: { system_administrator: '1' } }

    expect(response).to redirect_to(root_path)
    expect(target_user.person.account.platform_admin).to be_nil
  end

  it 'promotes a household member to owner with fresh privileged MFA and audits success' do
    membership = ensure_household_membership!(target_user.person.account, target_user.person, role: :member)
    sign_in(platform_user)
    authenticate_platform_totp(platform_user.person.account)

    expect do
      patch platform_promote_household_owner_path(membership.household, membership)
    end.to change { membership.reload.permissions_version }.by(1)

    expect(response).to redirect_to(platform_users_path)
    expect(membership).to be_owner
    event = SecurityAuditEvent.where(event_type: 'household_membership.role_updated').order(:id).last
    expect(event).to have_attributes(household: membership.household, actor_account: platform_user.person.account)
    expect(event.metadata).to include(
      'target_membership_id' => membership.id,
      'previous_role' => 'member',
      'new_role' => 'owner',
      'outcome' => 'success'
    )
  end

  it 'rejects owner promotion without fresh privileged MFA and audits the outcome' do
    membership = ensure_household_membership!(target_user.person.account, target_user.person, role: :member)
    sign_in(platform_user)

    expect do
      patch platform_promote_household_owner_path(membership.household, membership)
    end.to change {
      SecurityAuditEvent.where(event_type: 'household_owner_promotion.rejected').count
    }.by(1)

    expect(response).to have_http_status(:see_other)
    expect(membership.reload).to be_member
    event = SecurityAuditEvent.where(event_type: 'household_owner_promotion.rejected').order(:id).last
    expect(event.metadata).to include(
      'target_membership_id' => membership.id,
      'outcome' => 'rejected',
      'reason' => 'fresh_privileged_action_required'
    )
  end

  it 'rejects owner promotion when privileged MFA is stale' do
    membership = ensure_household_membership!(target_user.person.account, target_user.person, role: :member)
    sign_in(platform_user)
    authenticate_platform_totp(platform_user.person.account)

    travel HostedPrivilegedActionMfa::PRIVILEGED_ACTION_MFA_TTL + 1.minute do
      patch platform_promote_household_owner_path(membership.household, membership)
    end

    expect(response).to have_http_status(:see_other)
    expect(membership.reload).to be_member
    event = SecurityAuditEvent.where(event_type: 'household_owner_promotion.rejected').order(:id).last
    expect(event.metadata).to include('reason' => 'fresh_privileged_action_required')
  end

  it 'denies owner promotion to household owners without platform administration' do
    membership = ensure_household_membership!(target_user.person.account, target_user.person, role: :member)
    sign_in(household_owner)

    expect do
      patch platform_promote_household_owner_path(membership.household, membership)
    end.to change {
      SecurityAuditEvent.where(event_type: 'household_owner_promotion.rejected').count
    }.by(1)

    expect(response).to redirect_to(root_path)
    expect(membership.reload).to be_member
    event = SecurityAuditEvent.where(event_type: 'household_owner_promotion.rejected').order(:id).last
    expect(event.metadata).to include('reason' => 'platform_administrator_required', 'outcome' => 'rejected')
  end

  it 'rejects a membership outside the household in the promotion path' do
    target_membership = ensure_household_membership!(target_user.person.account, target_user.person, role: :member)
    foreign_household = Household.create!(name: 'Foreign Promotion', slug: "foreign-promotion-#{SecureRandom.hex(4)}")
    foreign_account = Account.create!(email: "foreign-promotion-#{SecureRandom.hex(4)}@example.test", status: :verified)
    foreign_person = Person.create!(
      household: foreign_household,
      account: foreign_account,
      name: 'Foreign Promotion Person',
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
    foreign_membership = foreign_household.household_memberships.create!(
      account: foreign_account,
      person: foreign_person,
      role: :member,
      status: :active
    )
    sign_in(platform_user)
    authenticate_platform_totp(platform_user.person.account)

    patch platform_promote_household_owner_path(target_membership.household, foreign_membership)

    expect(response).to have_http_status(:not_found)
    expect(foreign_membership.reload).to be_member
    event = SecurityAuditEvent.where(event_type: 'household_owner_promotion.rejected').order(:id).last
    expect(event.metadata).to include(
      'target_membership_id' => foreign_membership.id,
      'outcome' => 'rejected',
      'reason' => 'target_household_mismatch'
    )
  end

  def ensure_platform_admin!(account)
    platform_admin = account.platform_admin || PlatformAdmin.create!(account: account)
    platform_admin.active!
    platform_admin
  end

  def ensure_household_membership!(account, person, role:)
    membership = person.household.household_memberships.find_or_initialize_by(account: account)
    membership.person = person
    membership.role = role
    membership.status = :active
    membership.save!
    membership
  end

  def authenticate_platform_totp(account)
    secret = 'jbswy3dpehpk3pxp'
    visible_secret = RodauthApp.rodauth.allocate.send(:otp_hmac_secret, secret)
    AccountOtpKey.where(id: account.id).delete_all
    AccountOtpKey.create!(id: account.id, key: secret, last_use: 5.minutes.ago)
    post '/otp-auth', params: { otp: ROTP::TOTP.new(visible_secret).at(Time.current) }
  end

  describe Components::Platform::Users::IndexView, type: :component do
    fixtures :accounts, :people, :users

    it 'does not execute SQL while rendering preloaded users' do
      current_user = users(:admin)
      user_list = User.where(id: [current_user.id, users(:jane).id]).includes(person: :account).to_a
      access_summary = Admin::UserAccessSummaryQuery.new(users: user_list).call

      expect(count_queries do
        render_inline(described_class.new(
                        users: user_list,
                        current_user: current_user,
                        access_summary: access_summary
                      ))
      end).to eq(0)
    end

    def count_queries(&)
      count = 0
      subscriber = lambda do |_name, _started, _finished, _unique_id, payload|
        next if payload[:cached] || payload[:name] == 'SCHEMA'

        count += 1
      end

      ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
      count
    end
  end
end
