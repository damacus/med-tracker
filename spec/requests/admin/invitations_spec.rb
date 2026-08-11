# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin::Invitations' do
  fixtures :accounts, :people, :users

  let(:admin) { users(:admin) }
  let(:regular_user) { users(:jane) }

  describe 'POST /admin/invitations' do
    before { sign_in(admin) }

    it 'replaces the invitation form with Email feedback for invalid Turbo submissions' do
      post admin_invitations_path,
           params: { invitation: { email: '', membership_role: 'member', access_level: 'record' } },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      aggregate_failures do
        expect(response).to have_http_status(:unprocessable_content)
        expect(response.media_type).to eq('text/vnd.turbo-stream.html')
        expect(response.body).to include('target="admin_invitations"')
        expect(response.body).to include('aria-invalid="true"')
        expect(response.body).to include('aria-describedby="invitation_email_error"')
        expect(response.body).to include('id="invitation_email_error"')
      end
    end
  end

  describe 'DELETE /admin/invitations/:id' do
    context 'when authenticated as administrator' do
      before { sign_in(admin) }

      it 'destroys a pending invitation and redirects with a notice' do
        invitation = create_household_invitation_for_request

        delete admin_invitation_path(invitation)

        expect(response).to redirect_to(admin_invitations_path)
        expect(flash[:notice]).to eq('Invitation cancelled')
        expect(HouseholdInvitation.exists?(invitation.id)).to be false
      end

      it 'destroys an expired invitation and redirects with a notice' do
        invitation = create_household_invitation_for_request(:expired)

        delete admin_invitation_path(invitation)

        expect(response).to redirect_to(admin_invitations_path)
        expect(flash[:notice]).to eq('Invitation cancelled')
        expect(HouseholdInvitation.exists?(invitation.id)).to be false
      end

      it 'refuses to destroy an accepted invitation and redirects with an alert' do
        invitation = create_household_invitation_for_request(:accepted)

        delete admin_invitation_path(invitation)

        expect(response).to redirect_to(admin_invitations_path)
        expect(flash[:alert]).to eq('Accepted invitations cannot be cancelled')
        expect(HouseholdInvitation.exists?(invitation.id)).to be true
      end

      it 'returns turbo_stream for pending invitation destroy' do
        invitation = create_household_invitation_for_request

        delete admin_invitation_path(invitation), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

        expect(response).to have_http_status(:ok)
        expect(response.media_type).to eq('text/vnd.turbo-stream.html')
        expect(response.body).to include('target="flash"')
        expect(HouseholdInvitation.exists?(invitation.id)).to be false
      end
    end

    context 'when authenticated as non-administrator' do
      before { sign_in(regular_user) }

      it 'denies access' do
        invitation = create_household_invitation_for_request

        delete admin_invitation_path(invitation)

        expect(response).to redirect_to(root_path)
        expect(HouseholdInvitation.exists?(invitation.id)).to be true
      end
    end
  end

  def create_household_invitation_for_request(*traits)
    household = Household.find_by!(slug: default_request_household_slug)
    inviter = household.household_memberships.owner.active.first || household.household_memberships.active.first
    create(:household_invitation, *traits, household: household, invited_by_membership: inviter)
  end
end
