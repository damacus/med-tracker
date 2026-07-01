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

    patch platform_user_path(target_user), params: { platform_user: { system_administrator: '0' } }

    expect(response).to redirect_to(platform_users_path)
    expect(target_user.person.account.platform_admin.reload).to be_disabled
    expect(target_membership.reload).to be_owner
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
end
