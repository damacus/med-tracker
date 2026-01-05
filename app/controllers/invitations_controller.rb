# frozen_string_literal: true

class InvitationsController < ApplicationController
  allow_unauthenticated_access
  layout false

  def accept
    @invitation = Invitation.pending.find_by!(token: params[:token])

    view = Components::Invitations::AcceptView.new(invitation: @invitation)
    render Components::Layouts::AuthLayout.new(title: 'Accept Invitation - MedTracker', component: view)
  end
end
