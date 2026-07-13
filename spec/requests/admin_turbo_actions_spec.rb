# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin turbo actions' do
  fixtures :accounts, :people, :users, :carer_relationships

  let(:admin) { users(:admin) }

  before do
    FixtureHouseholdSetup.apply!
    sign_in(admin)
  end

  describe 'POST /admin/users/:id/activate' do
    it 'returns turbo_stream and updates the user row and flash' do
      user = users(:jane)
      user.deactivate!

      post activate_admin_user_path(user), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"user_#{user.id}\"")
      expect(response.body).to include('target="flash"')
      expect(user.reload).to be_active
    end

    it 'reactivates a deactivated user and redirects with a notice (HTML)' do
      user = users(:jane)
      user.deactivate!

      post activate_admin_user_path(user)

      expect(response).to redirect_to(admin_users_path)
      expect(user.reload).to be_active
    end
  end

  describe 'DELETE /admin/users/:id' do
    it 'returns turbo_stream and updates the user row and flash' do
      user = users(:jane)

      delete admin_user_path(user), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"user_#{user.id}\"")
      expect(response.body).to include('target="flash"')
      expect(user.reload).not_to be_active
    end

    context 'when targeting the currently signed-in admin' do
      it 'refuses to deactivate self and returns unprocessable content (turbo)' do
        delete admin_user_path(admin), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

        expect(response).to have_http_status(:unprocessable_content)
        expect(response.body).to include('target="flash"')
        expect(admin.reload).to be_active
      end

      it 'refuses to deactivate self and redirects with an alert (HTML)' do
        delete admin_user_path(admin)

        expect(response).to redirect_to(admin_users_path)
        expect(admin.reload).to be_active
      end
    end
  end

  describe 'POST /admin/users/:id/verify' do
    it 'returns turbo_stream and updates the user row and flash' do
      user = users(:jane)
      user.person.account.update!(status: :unverified)

      post verify_admin_user_path(user), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"user_#{user.id}\"")
      expect(response.body).to include('target="flash"')
      expect(user.person.account.reload).to be_verified
    end

    context 'when the user has no account' do
      it 'returns unprocessable content and a missing-account alert (turbo)' do
        user = users(:jane)
        user.person.update!(account: nil)

        post verify_admin_user_path(user), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

        expect(response).to have_http_status(:unprocessable_content)
        expect(response.body).to include('target="flash"')
      end
    end
  end

  describe 'POST /admin/carer_relationships/:id/activate' do
    it 'returns turbo_stream and updates the relationship row and flash' do
      relationship = carer_relationships(:inactive_relationship)

      post activate_admin_carer_relationship_path(relationship), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"carer_relationship_#{relationship.id}\"")
      expect(response.body).to include('target="flash"')
      expect(relationship.reload).to be_active
    end

    it 'returns unprocessable content when a manual grant conflicts with activation' do
      relationship = carer_relationships(:inactive_relationship)
      membership = relationship.household.household_memberships.find_by!(person: relationship.carer)
      relationship.household.person_access_grants.where(
        household_membership: membership,
        person: relationship.patient
      ).delete_all
      grant = relationship.household.person_access_grants.create!(
        household_membership: membership,
        person: relationship.patient,
        access_level: :view,
        relationship_type: :family_member,
        granted_by_membership: membership
      )

      post activate_admin_carer_relationship_path(relationship), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('target="flash"')
      expect(response.body).to include('existing manual grant')
      expect(relationship.reload).not_to be_active
      expect(grant.reload).to have_attributes(access_level: 'view', revoked_at: nil)
    end

    it 'returns unprocessable content for an unsupported legacy relationship type' do
      relationship = carer_relationships(:inactive_relationship)
      relationship.update!(relationship_type: 'guardian')

      post activate_admin_carer_relationship_path(relationship), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('target="flash"')
      expect(response.body).to include('unsupported carer relationship type')
      expect(relationship.reload).not_to be_active
    end
  end

  describe 'DELETE /admin/carer_relationships/:id' do
    it 'returns turbo_stream and updates the relationship row and flash' do
      relationship = carer_relationships(:jane_cares_for_child)
      membership = relationship.household.household_memberships.find_by!(person: relationship.carer)
      relationship.household.person_access_grants.find_by!(
        household_membership: membership,
        person: relationship.patient
      ).update!(carer_relationship: relationship)

      delete admin_carer_relationship_path(relationship), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"carer_relationship_#{relationship.id}\"")
      expect(response.body).to include('target="flash"')
      expect(relationship.reload).not_to be_active
    end

    it 'returns unprocessable content without revoking an unowned grant' do
      relationship = carer_relationships(:jane_cares_for_child)
      membership = relationship.household.household_memberships.find_by!(person: relationship.carer)
      manual_grant = relationship.household.person_access_grants.find_by!(
        household_membership: membership,
        person: relationship.patient
      )
      manual_grant.update!(carer_relationship: nil)

      delete admin_carer_relationship_path(relationship), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('target="flash"')
      expect(response.body).to include('unowned grant')
      expect(relationship.reload).to be_active
      expect(manual_grant.reload.revoked_at).to be_nil
    end
  end
end
