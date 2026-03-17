# frozen_string_literal: true

class InvitationsController < ApplicationController
  allow_unauthenticated_access
  layout false

  def accept
    token_value = params[:token].to_s
    digest = Invitation.digest(token_value)
    @invitation = Invitation.pending.where.not(role: Invitation.roles[:minor]).find_by(token_digest: digest)

    unless @invitation
      render plain: 'This invitation link is invalid or has expired.', status: :not_found
      return
    end

    view = Components::Invitations::AcceptView.new(invitation: @invitation, token: token_value)
    render Components::Layouts::AuthLayout.new(title: 'Accept Invitation - MedTracker', component: view)
  end
end
