# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Household operational boundaries' do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:user) { users(:admin) }
  let(:household) { user.person.household }

  before do
    household.household_memberships.find_or_create_by!(account: user.person.account) do |membership|
      membership.person = user.person
      membership.role = :owner
      membership.status = :active
    end
  end

  it 'denies web tenant selection for every unavailable lifecycle state' do
    %i[held offboarded purged].each do |state|
      household.update!(lifecycle_state: state)
      sign_in(user)

      get dashboard_path(household_slug: household.slug)

      expect(response).to redirect_to('/login')
      expect(flash[:alert]).to eq('You are not authorized to perform this action.')
      expect(Current.household).to be_nil
    end
  end

  it 'denies API tenant selection for every unavailable lifecycle state' do
    login_data = api_login(user, household_id: household.id)
    %i[held offboarded purged].each do |state|
      household.update!(lifecycle_state: state)

      get api_v1_household_medications_path(household.id),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:unauthorized)
      expect(response.parsed_body).not_to have_key('data')
    end
  end

  it 'does not issue or refresh API sessions for an unavailable household' do
    login_data = api_login(user, household_id: household.id)

    %i[held offboarded purged].each do |state|
      household.update!(lifecycle_state: state)

      expect do
        post api_v1_auth_login_path,
             params: { email: user.email_address, password: 'password', household_id: household.id },
             as: :json
      end.not_to change(ApiSession, :count)

      expect(response).to have_http_status(:unauthorized)
    end

    household.update!(lifecycle_state: :held)
    post api_v1_auth_refresh_path,
         params: { refresh_token: login_data.fetch('refresh_token') },
         as: :json

    expect(response).to have_http_status(:unauthorized)
  end

  it 'denies support mode for a held household' do
    platform_account = Account.create!(email: 'held-support@example.test', status: :verified)
    platform_person = household.people.create!(
      account: platform_account,
      name: 'Held Support Operator',
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
    platform_user = User.create!(
      person: platform_person,
      email_address: platform_account.email,
      password: 'password'
    )
    platform_admin = PlatformAdmin.create!(account: platform_account)
    SupportAccessSession.create!(
      platform_admin: platform_admin,
      household: household,
      reason: 'Operational support check',
      mfa_verified_at: Time.current,
      starts_at: Time.current,
      expires_at: 15.minutes.from_now
    )
    %i[held offboarded purged].each do |state|
      household.update!(lifecycle_state: state)
      sign_in(platform_user)

      get admin_root_path(household_slug: household.slug)

      expect(response).to redirect_to('/login')
      expect(response.body).not_to include(household.slug)
      expect(Current.household).to be_nil
    end
  end
end
