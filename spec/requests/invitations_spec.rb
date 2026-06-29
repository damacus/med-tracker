# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Invitations' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :carer_relationships

  describe 'GET /invitations/accept' do
    it 'rejects unknown invitation tokens' do
      get accept_invitation_path(token: SecureRandom.hex(32))

      expect(response).to have_http_status(:not_found)
      expect(response.body).to include('This invitation link is invalid or has expired.')
    end

    it 'rejects an old token after an invitation is resent' do
      household, membership = household_bundle(email: 'resend-owner@example.test', name: 'Resend Invitation')
      invitation = create(:household_invitation, household: household, invited_by_membership: membership)
      original_token = invitation.token

      invitation.resend!

      get accept_invitation_path(token: original_token)

      expect(response).to have_http_status(:not_found)
      expect(response.body).to include('This invitation link is invalid or has expired.')
    end
  end

  describe 'POST /create-account with an invitation' do
    it 'pins the created account email to the invitation email' do
      household, membership = household_bundle(email: 'pinned-owner@example.test', name: 'Pinned Invitation')
      invitation = create(
        :household_invitation,
        household: household,
        invited_by_membership: membership,
        email: 'invited.email@example.com'
      )

      expect do
        post create_account_path,
             params: {
               invitation_token: invitation.token,
               name: 'Pinned Invitee',
               date_of_birth: '1985-05-15',
               email: 'attacker.supplied@example.com',
               password: 'SecureP@ssword123!',
               'password-confirm': 'SecureP@ssword123!'
             }
      end.to change(Account, :count).by(1)

      expect(Account.exists?(email: 'invited.email@example.com')).to be(true)
      expect(Account.exists?(email: 'attacker.supplied@example.com')).to be(false)
      expect(invitation.reload.accepted_at).to be_present
    end

    it 'rejects accepted invitation tokens without creating another account' do
      household, membership = household_bundle(email: 'accepted-owner@example.test', name: 'Accepted Invitation')
      invitation = create(
        :household_invitation,
        household: household,
        invited_by_membership: membership,
        email: 'already.accepted@example.com',
        accepted_at: Time.current
      )

      expect do
        post create_account_path,
             params: {
               invitation_token: invitation.token,
               name: 'Already Accepted',
               date_of_birth: '1985-05-15',
               email: 'already.accepted@example.com',
               password: 'SecureP@ssword123!',
               'password-confirm': 'SecureP@ssword123!'
             }
      end.not_to change(Account, :count)

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('There was an error creating your account')
    end

    it 'rejects expired invitation tokens without creating an account' do
      household, membership = household_bundle(email: 'expired-owner@example.test', name: 'Expired Invitation')
      invitation = create(
        :household_invitation,
        household: household,
        invited_by_membership: membership,
        email: 'expired.invitee@example.com',
        expires_at: 1.day.ago
      )

      expect do
        post create_account_path,
             params: {
               invitation_token: invitation.token,
               name: 'Expired Invitee',
               date_of_birth: '1985-05-15',
               email: 'expired.invitee@example.com',
               password: 'SecureP@ssword123!',
               'password-confirm': 'SecureP@ssword123!'
             }
      end.not_to change(Account, :count)

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('There was an error creating your account')
    end

    it 'links accepted parent invitations to selected existing dependents' do
      dependent = people(:child_patient)
      household, membership = household_bundle(email: 'parent-owner@example.test', name: 'Parent Invitation')
      dependent.update!(household: household)
      invitation = create_household_invitation_with_grant(
        household: household,
        membership: membership,
        email: 'accepted.parent@example.com',
        dependent: dependent,
        grant: { access_level: :manage, relationship_type: :parent }
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
      accepted_membership = household.household_memberships.find_by!(account: parent.account)
      grant = household.person_access_grants.find_by!(household_membership: accepted_membership, person: dependent)
      expect(relationship.relationship_type).to eq('parent')
      expect(relationship.active).to be true
      expect(accepted_membership.role).to eq('member')
      expect(grant).to have_attributes(access_level: 'manage', relationship_type: 'parent')
      expect(invitation.reload.accepted_at).to be_present
    end

    it 'links accepted carer invitations to selected existing dependents' do
      dependent = people(:child_user_person)
      household, membership = household_bundle(email: 'carer-owner@example.test', name: 'Carer Invitation')
      dependent.update!(household: household)
      invitation = create_household_invitation_with_grant(
        household: household,
        membership: membership,
        email: 'accepted.carer@example.com',
        dependent: dependent,
        grant: { access_level: :record, relationship_type: :professional }
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
      accepted_membership = household.household_memberships.find_by!(account: carer.account)
      grant = household.person_access_grants.find_by!(household_membership: accepted_membership, person: dependent)
      expect(relationship.relationship_type).to eq('professional_carer')
      expect(relationship.active).to be true
      expect(accepted_membership.role).to eq('member')
      expect(grant).to have_attributes(access_level: 'record', relationship_type: 'professional')
      expect(invitation.reload.accepted_at).to be_present
    end
  end

  def household_bundle(email:, name:)
    account = Account.create!(email: email, status: :verified)
    household = Household.create_with_owner!(
      name: name,
      owner_account: account,
      owner_person_attributes: {
        name: "#{name} Owner",
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      }
    )
    [household, household.household_memberships.sole]
  end

  def create_household_invitation_with_grant(household:, membership:, email:, dependent:, grant:)
    invitation = create(
      :household_invitation,
      household: household,
      invited_by_membership: membership,
      email: email
    )
    invitation.household_invitation_grants.create!(
      household: household,
      person: dependent,
      access_level: grant.fetch(:access_level),
      relationship_type: grant.fetch(:relationship_type)
    )
    invitation
  end
end
