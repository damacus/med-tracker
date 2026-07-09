# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 medications' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :carer_relationships, :medications,
           :person_medications

  let(:user) { users(:admin) }

  describe 'GET /api/v1/households/:household_id/medications collection' do
    it 'records high-risk API access without persisting the bearer token', :aggregate_failures do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')
      access_token = login_data.fetch('access_token')

      expect do
        get api_v1_household_medications_path(household_id),
            headers: api_auth_headers(access_token),
            as: :json
      end.to change { SecurityAuditEvent.where(event_type: 'api.request').count }.by(1)

      event = SecurityAuditEvent.where(event_type: 'api.request').order(:created_at).last
      expect(event.audit_context).to include(
        'authentication_method' => 'api_session',
        'active_role' => 'owner',
        'policy_class' => 'MedicationPolicy',
        'policy_query' => 'index?'
      )
      expect(event.metadata).to include(
        'http_method' => 'GET',
        'controller' => 'api/v1/medications',
        'action' => 'index',
        'outcome' => 'success',
        'status' => 200
      )
      expect(event.attributes.to_json).not_to include(access_token)
    end

    it 'returns only medications in the signed-in user scope' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      get api_v1_household_medications_path(household_id),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:ok)

      returned_ids = response.parsed_body.fetch('data').map { |m| m.fetch('id') }
      expect(returned_ids).to include(medications(:paracetamol).id)
    end

    it 'omits unrelated medications for delegated members with manage grants' do
      scoped_user = users(:jane)
      household = scoped_user.person.household
      membership = household.household_memberships.find_or_create_by!(account: scoped_user.person.account) do |record|
        record.person = scoped_user.person
        record.role = :member
        record.status = :active
      end
      membership.update!(person: scoped_user.person, role: :member, status: :active)
      visible_person = create(:person, household: household, name: 'Delegated Medication Visible')
      hidden_person = create(:person, household: household, name: 'Delegated Medication Hidden')
      visible_medication = create(:medication, household: household, location: locations(:home), name: 'Visible Stock')
      hidden_medication = create(:medication, household: household, location: locations(:home), name: 'Hidden Stock')
      create(:person_medication, household: household, person: visible_person, medication: visible_medication)
      create(:person_medication, household: household, person: hidden_person, medication: hidden_medication)
      login_data = api_login(scoped_user, household_id: household.id)
      household.person_access_grants.where(household_membership: membership).destroy_all
      household.person_access_grants.create!(
        household_membership: membership,
        person: visible_person,
        access_level: :manage,
        relationship_type: :family_member,
        granted_by_membership: membership
      )

      get api_v1_household_medications_path(household.id),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:ok)
      returned_ids = response.parsed_body.fetch('data').map { |m| m.fetch('id') }
      expect(returned_ids).to include(visible_medication.id)
      expect(returned_ids).not_to include(hidden_medication.id)
    end
  end

  describe 'GET /api/v1/households/:household_id/medications/:id' do
    it 'returns the medication when in scope' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      get api_v1_household_medication_path(household_id, medications(:paracetamol).id),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.dig('data', 'id')).to eq(medications(:paracetamol).id)
    end

    it 'returns not found for a medication outside scope' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')
      other_household = Household.create!(name: 'Other', slug: 'other')
      other_location = Location.create!(household: other_household, name: 'loc')
      other_medication = Medication.create!(household: other_household, location: other_location, name: 'Other')

      get api_v1_household_medication_path(household_id, other_medication.id),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:not_found)
    end
  end

  describe 'POST /api/v1/households/:household_id/medications' do
    it 'creates a medication with valid attributes' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      post api_v1_household_medications_path(household_id),
           params: {
             medication: {
               name: 'New API Medication', location_id: locations(:home).id,
               dose_amount: 5, dose_unit: 'ml', current_supply: 100, reorder_threshold: 10
             }
           },
           headers: api_auth_headers(login_data.fetch('access_token')), as: :json

      expect(response).to have_http_status(:created)
      expect(response.parsed_body.dig('data', 'name')).to eq('New API Medication')
    end

    it 'keeps delegated api-created medications visible to the creating membership' do
      scoped_user = users(:jane)
      household = scoped_user.person.household
      membership = household.household_memberships.find_or_create_by!(account: scoped_user.person.account) do |record|
        record.person = scoped_user.person
        record.role = :member
        record.status = :active
      end
      membership.update!(person: scoped_user.person, role: :member, status: :active)
      managed_person = create(:person, household: household, name: 'Delegated Medication Creator')
      location = create(:location, household: household, name: 'Delegated API Shelf')
      household.person_access_grants.where(household_membership: membership).destroy_all
      household.person_access_grants.create!(
        household_membership: membership,
        person: managed_person,
        access_level: :manage,
        relationship_type: :family_member,
        granted_by_membership: membership
      )
      login_data = api_login(scoped_user, household_id: household.id)

      post api_v1_household_medications_path(household.id),
           params: {
             medication: {
               name: 'Delegated API Medication', location_id: location.id,
               dose_amount: 5, dose_unit: 'ml', current_supply: 100, reorder_threshold: 10
             }
           },
           headers: api_auth_headers(login_data.fetch('access_token')), as: :json

      expect(response).to have_http_status(:created)

      created_id = response.parsed_body.dig('data', 'id')
      get api_v1_household_medication_path(household.id, created_id),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.dig('data', 'id')).to eq(created_id)

      get api_v1_household_medications_path(household.id),
          headers: api_auth_headers(login_data.fetch('access_token')),
          as: :json

      returned_ids = response.parsed_body.fetch('data').map { |medication| medication.fetch('id') }
      expect(returned_ids).to include(created_id)
    end

    it 'returns unprocessable content with invalid attributes' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      post api_v1_household_medications_path(household_id),
           params: { medication: { name: '', location_id: locations(:home).id } },
           headers: api_auth_headers(login_data.fetch('access_token')), as: :json

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.dig('error', 'errors')).to include('name')
    end
  end

  describe 'PATCH /api/v1/households/:household_id/medications/:id' do
    it 'attributes PaperTrail changes to the API session and authorization decision', :aggregate_failures do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')
      access_token = login_data.fetch('access_token')
      medication = medications(:paracetamol)

      patch api_v1_household_medication_path(household_id, medication.id),
            params: { medication: { current_supply: 91 } },
            headers: api_auth_headers(access_token),
            as: :json

      version = PaperTrail::Version.where(item: medication).order(:created_at).last
      expect(version.audit_context).to include(
        'authentication_method' => 'api_session',
        'active_role' => 'owner',
        'policy_class' => 'MedicationPolicy',
        'policy_query' => 'update?'
      )
      expect(version.audit_context.fetch('session_reference')).to match(/\Aapi_session:\d+\z/)
      expect(version.object_changes).to be_present
      expect(version.attributes.to_json).not_to include(access_token)
    end

    it 'updates a medication with valid attributes' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      patch api_v1_household_medication_path(household_id, medications(:paracetamol).id),
            params: { medication: { current_supply: 90 } },
            headers: api_auth_headers(login_data.fetch('access_token')), as: :json

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.dig('data', 'current_supply')).to eq('90.0')
    end

    it 'returns unprocessable content with invalid attributes' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')

      patch api_v1_household_medication_path(household_id, medications(:paracetamol).id),
            params: { medication: { name: '' } },
            headers: api_auth_headers(login_data.fetch('access_token')), as: :json

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.dig('error', 'errors')).to include('name')
    end

    it 'rejects a location from another household' do
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')
      medication = medications(:paracetamol)
      original_location = medication.location
      other_household = Household.create!(name: 'Medication Location Boundary', slug: 'med-location-boundary')
      other_location = Location.create!(household: other_household, name: 'Other Household Cabinet')

      patch api_v1_household_medication_path(household_id, medication.id),
            params: { medication: { location_id: other_location.id } },
            headers: api_auth_headers(login_data.fetch('access_token')),
            as: :json

      expect(response).to have_http_status(:not_found)
      expect(medication.reload.location).to eq(original_location)
    end
  end
end
