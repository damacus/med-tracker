# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Invitations' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :carer_relationships

  describe 'GET /invitations/accept' do
    it 'rejects existing minor invitation tokens' do
      invitation = Invitation.new(email: 'child.invite@example.com', role: :minor)
      invitation.save!(validate: false)

      get accept_invitation_path(token: invitation.token)

      expect(response).to have_http_status(:not_found)
      expect(response.body).to include('This invitation link is invalid or has expired.')
    end

    it 'rejects an old token after an invitation is resent' do
      invitation = create(:invitation)
      original_token = invitation.token

      invitation.resend!

      get accept_invitation_path(token: original_token)

      expect(response).to have_http_status(:not_found)
      expect(response.body).to include('This invitation link is invalid or has expired.')
    end
  end

  describe 'POST /create-account with an invitation' do
    it 'links accepted parent invitations to selected existing dependents' do
      dependent = people(:child_patient)
      invitation = create(
        :invitation,
        email: 'accepted.parent@example.com',
        role: :parent,
        dependent_ids: [dependent.id]
      )

      expect do
        post create_account_path,
             params: {
               invitation_token: invitation.token,
               name: 'Accepted Parent',
               date_of_birth: '1985-05-15',
               email: 'accepted.parent@example.com',
               password: 'SecureP@ssword123!',
               'password-confirm': 'SecureP@ssword123!'
             }
      end.to change(CarerRelationship, :count).by(1)

      parent = Person.find_by!(email: 'accepted.parent@example.com')
      relationship = CarerRelationship.find_by!(carer: parent, patient: dependent)
      expect(relationship.relationship_type).to eq('parent')
      expect(relationship.active).to be true
      expect(invitation.reload.accepted_at).to be_present
    end

    it 'links accepted carer invitations to selected existing dependents' do
      dependent = people(:child_user_person)
      invitation = create(
        :invitation,
        email: 'accepted.carer@example.com',
        role: :carer,
        dependent_ids: [dependent.id]
      )

      expect do
        post create_account_path,
             params: {
               invitation_token: invitation.token,
               name: 'Accepted Carer',
               date_of_birth: '1985-05-15',
               email: 'accepted.carer@example.com',
               password: 'SecureP@ssword123!',
               'password-confirm': 'SecureP@ssword123!'
             }
      end.to change(CarerRelationship, :count).by(1)

      carer = Person.find_by!(email: 'accepted.carer@example.com')
      relationship = CarerRelationship.find_by!(carer: carer, patient: dependent)
      expect(relationship.relationship_type).to eq('professional_carer')
      expect(relationship.active).to be true
      expect(invitation.reload.accepted_at).to be_present
    end
  end
end
