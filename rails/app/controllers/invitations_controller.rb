# frozen_string_literal: true

class InvitationsController < ApplicationController
  allow_unauthenticated_access
  layout false
  skip_after_action :verify_pundit_authorization

  def accept
    token_value = params.expect(:token).to_s
    @invitation = HouseholdInvitations::TokenResolver.call(token_value)

    unless @invitation
      render plain: 'This invitation link is invalid or has expired.', status: :not_found
      return
    end

    TenantContext.with(account: nil, household: @invitation.household, request_id: request.request_id) do
      view = Components::Invitations::AcceptView.new(invitation: @invitation, token: token_value)
      render Components::Layouts::AuthLayout.new(title: 'Accept Invitation - MedTracker', component: view)
    end
  end
end
