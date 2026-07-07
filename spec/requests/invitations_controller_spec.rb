# frozen_string_literal: true

require 'rails_helper'

RSpec.describe InvitationsController do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :carer_relationships

  describe 'GET #accept' do
    let(:account) { Account.create!(email: 'valid-owner@example.test', status: :verified) }
    let(:household) do
      Household.create_with_owner!(
        name: 'Valid Token Household',
        owner_account: account,
        owner_person_attributes: {
          name: 'Valid Owner',
          date_of_birth: 30.years.ago.to_date,
          person_type: :adult,
          has_capacity: true
        }
      )
    end
    let(:membership) { household.household_memberships.sole }
    let(:invitation) do
      create(:household_invitation, household: household, invited_by_membership: membership)
    end

    it 'renders the accept invitation view for a valid token' do
      get accept_invitation_path(token: invitation.token)

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('Accept Invitation - MedTracker')
    end

    it 'renders a not found message for an invalid or expired token' do
      get accept_invitation_path(token: 'invalid_token')

      expect(response).to have_http_status(:not_found)
      expect(response.body).to include('This invitation link is invalid or has expired.')
    end

    it 'returns a 400 Bad Request when token is missing' do
      get accept_invitation_path

      expect(response).to have_http_status(:bad_request)
    end
  end
end
