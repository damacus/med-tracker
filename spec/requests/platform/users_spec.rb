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
