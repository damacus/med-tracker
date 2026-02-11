# frozen_string_literal: true

module Admin
  class InvitationsController < ApplicationController
    def index
      authorize :invitation, :index?

      render Components::Admin::Invitations::IndexView.new
    end

    def create
      authorize :invitation, :create?

      @invitation = Invitation.new(invitation_params)

      if @invitation.save
        InvitationMailer.with(invitation: @invitation).invite.deliver_later
        redirect_back_or_to admin_invitations_path, notice: 'Invitation sent'
      else
        render Components::Admin::Invitations::IndexView.new(invitation: @invitation), status: :unprocessable_content
      end
    end

    private

    def invitation_params
      params.expect(invitation: %i[email role])
    end
  end
end
