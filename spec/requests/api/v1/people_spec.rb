# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 people' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :carer_relationships

  let(:user) { users(:jane) }

  describe 'GET /api/v1/households/:household_id/people collection' do
    it 'returns only people in the signed-in user scope' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      get api_v1_household_people_path(household_id),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:ok)

      returned_ids = response.parsed_body.fetch('data').map { |person| person.fetch('id') }
      expect(returned_ids).to include(people(:jane).id, people(:child_patient).id)
      expect(returned_ids).not_to include(people(:john).id)
    end

    it 'returns a structured error for an invalid updated_since filter' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      get api_v1_household_people_path(household_id),
          params: { updated_since: 'not-a-timestamp' },
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.dig('error', 'code')).to eq('unprocessable_content')
    end
  end

  describe 'GET /api/v1/households/:household_id/people/:id' do
    it 'returns forbidden for a person outside scope' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      get api_v1_household_person_path(household_id, people(:john)),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:not_found)
    end
  end

  describe 'GET /api/v1/households/:household_id/people household grants' do
    it 'returns only people granted to the session membership inside the routed household' do
      account = user.person.account
      household = user.person.household
      other_household = Household.create!(name: 'Other API Household', slug: 'other-api-household')

      granted_person = Person.create!(
        household: household,
        name: 'Granted Alex',
        date_of_birth: 8.years.ago.to_date,
        person_type: :minor
      )
      hidden_person = Person.create!(
        household: household,
        name: 'Hidden Alex',
        date_of_birth: 9.years.ago.to_date,
        person_type: :minor
      )
      other_household_person = Person.create!(
        household: other_household,
        name: 'Other Alex',
        date_of_birth: 10.years.ago.to_date,
        person_type: :minor
      )
      membership = household.household_memberships.find_or_create_by!(
        account: account,
        person: people(:jane)
      ) do |record|
        record.role = :owner
        record.status = :active
      end
      household.person_access_grants.create!(
        household_membership: membership,
        person: people(:jane),
        access_level: :manage,
        relationship_type: :self,
        granted_by_membership: membership
      )
      household.person_access_grants.create!(
        household_membership: membership,
        person: granted_person,
        access_level: :view,
        relationship_type: :parent,
        granted_by_membership: membership
      )

      login_data = api_login(user, household_id: household.id)

      allow(TenantContext).to receive(:with).and_call_original
      allow(TenantContext).to receive(:set_membership!).and_call_original

      get "/api/v1/households/#{household.id}/people",
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(TenantContext).to have_received(:with).with(
        account: account,
        household: nil,
        request_id: kind_of(String)
      )
      expect(TenantContext).to have_received(:set_membership!).with(membership).at_least(:once)
      expect(response).to have_http_status(:ok)
      returned_ids = response.parsed_body.fetch('data').map { |person| person.fetch('id') }
      expect(returned_ids).to contain_exactly(people(:jane).id, granted_person.id)
      expect(returned_ids).not_to include(hidden_person.id, other_household_person.id)
    end

    it 'rejects a token used against a different household route' do
      account = user.person.account
      household = user.person.household
      other_household = Household.create!(name: 'Secondary API Household', slug: 'secondary-api-household')
      household.household_memberships.find_or_create_by!(account: account, person: people(:jane)) do |record|
        record.role = :owner
        record.status = :active
      end

      login_data = api_login(user, household_id: household.id)

      get "/api/v1/households/#{other_household.id}/people",
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:forbidden)
      expect(response.parsed_body.dig('error', 'code')).to eq('forbidden')
    end
  end
end
