# frozen_string_literal: true

class InvitationsController < ApplicationController
  allow_unauthenticated_access
  layout false

  def accept
    @invitation = Invitation.pending.find_by(token: params[:token])

    unless @invitation
      render plain: 'This invitation link is invalid or has expired.', status: :not_found
      return
    end

    view = Components::Invitations::AcceptView.new(invitation: @invitation)
    render Components::Layouts::AuthLayout.new(title: 'Accept Invitation - MedTracker', component: view)
  end
end
