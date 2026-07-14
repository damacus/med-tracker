# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin ambiguous person access grants' do
  fixtures :accounts, :people, :users, :carer_relationships

  let(:admin) { users(:admin) }
  let(:queue_data) do
    household = Household.find_by!(slug: default_request_household_slug)
    carer = household.people.create!(name: 'Queue Carer', date_of_birth: 35.years.ago.to_date)
    patient = household.people.create!(name: 'Queue Patient', date_of_birth: 38.years.ago.to_date,
                                       person_type: :adult, has_capacity: true)
    account = Account.create!(email: 'queue-carer@example.test', status: :verified)
    membership = household.household_memberships.create!(account: account, person: carer,
                                                         role: :member, status: :active)
    CarerRelationship.create!(household: household, carer: carer, patient: patient,
                              relationship_type: :parent, active: true)
    grant = PersonAccessGrant.create!(household: household, household_membership: membership, person: patient,
                                      access_level: :manage, relationship_type: :professional)
    { grant: grant, carer: carer, patient: patient }
  end

  context 'when authenticated' do
    before { sign_in(admin) }

    it 'lists eligible grants using household-safe fields after fresh MFA' do
      allow(ApiAuthState).to receive(:web_session_oidc_mfa_verified?).and_return(true)
      queue_data

      get admin_ambiguous_person_access_grants_path

      expect(response).to have_http_status(:ok)
      expect(response.body).to include(queue_data.fetch(:grant).id.to_s, queue_data.fetch(:carer).name,
                                       queue_data.fetch(:patient).name, 'Manage', 'Parent')
      expect(response.body).not_to include('queue-carer@example.test', 'date_of_birth', 'audit')
    end

    it 'requires fresh MFA even when hosted admin MFA is disabled' do
      get admin_ambiguous_person_access_grants_path

      expect(response).to redirect_to(profile_path)
      expect(response.body).not_to include(queue_data.fetch(:grant).id.to_s)
    end

    it 'does not send an unauthorized member to MFA setup' do
      sign_in(users(:jane))

      get admin_ambiguous_person_access_grants_path

      expect(response).to redirect_to(root_path)
      expect(flash[:alert]).not_to include('Set up MFA or a passkey')
    end
  end

  context 'without authentication' do
    it 'redirects an unauthenticated visitor to login' do
      get admin_ambiguous_person_access_grants_path

      expect(response).to redirect_to(login_path)
    end
  end
end
