# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::PeopleController do
  let(:person) { instance_double(Person, id: 1) }
  let(:household) { instance_double(Household, id: 1) }
  let(:membership) { instance_double(HouseholdMembership, active?: true, household_id: 1, person: person) }
  let(:api_session) { instance_double(ApiSession, account: double(verified?: true), active_for_membership?: true, household_membership: membership, revoked_at: nil, access_expires_at: 1.day.from_now) }

  before do
    allow(controller).to receive(:authenticate_api_request!).and_return(true)
    allow(controller).to receive(:bind_api_session_context!).and_return(true)
    allow(controller).to receive(:bind_api_household_context!).and_return(true)

    allow(controller).to receive(:current_household).and_return(household)
    allow(controller).to receive(:current_membership).and_return(membership)
    allow(controller).to receive(:authorize).and_return(true)
  end

  describe 'GET #index' do
    it 'authorizes Person and renders collection' do
      scope = double('scope')
      allow(controller).to receive(:policy_scope).with(Person).and_return(scope)
      expect(controller).to receive(:authorize).with(Person)

      expect(controller).to receive(:render_collection).with(
        scope,
        serializer: PersonSerializer,
        includes: %i[locations notification_preference]
      )

      get :index, params: { household_id: 1 }
    end
  end

  describe 'GET #show' do
    it 'authorizes the person and renders the resource' do
      scope = double('scope')
      allow(controller).to receive(:policy_scope).with(Person).and_return(scope)
      allow(scope).to receive(:includes).with(:locations, :notification_preference).and_return(scope)
      allow(scope).to receive(:find).with('1').and_return(person)

      expect(controller).to receive(:authorize).with(person)
      expect(controller).to receive(:render_resource).with(
        person,
        serializer: PersonSerializer
      )

      get :show, params: { household_id: 1, id: 1 }
    end
  end

  describe 'POST #create' do
    let(:new_person) { instance_double(Person, save: true, reload: person) }

    before do
      allow(Person).to receive(:new).and_return(new_person)
      allow(new_person).to receive(:household=).with(household)
    end

    it 'creates a new person and renders the resource' do
      expect(controller).to receive(:authorize).with(new_person)

      allow(controller).to receive(:assign_created_person_carer_relationship)
      allow(controller).to receive(:grant_created_person_access)

      expect(controller).to receive(:render_resource).with(
        person,
        serializer: PersonSerializer,
        status: :created
      )

      post :create, params: { household_id: 1, person: { name: 'Test', date_of_birth: '2000-01-01', email: 'test@test.com', person_type: 'adult', has_capacity: true } }
    end
  end

  describe 'PATCH #update' do
    it 'updates the person and renders the resource' do
      scope = double('scope')
      allow(controller).to receive(:policy_scope).with(Person).and_return(scope)
      allow(scope).to receive(:includes).with(:locations, :notification_preference).and_return(scope)
      allow(scope).to receive(:find).with('1').and_return(person)

      allow(person).to receive(:update).and_return(true)
      allow(person).to receive(:reload).and_return(person)

      expect(controller).to receive(:authorize).with(person)
      expect(controller).to receive(:render_resource).with(
        person,
        serializer: PersonSerializer
      )

      patch :update, params: { household_id: 1, id: 1, person: { name: 'Updated' } }
    end
  end
end
