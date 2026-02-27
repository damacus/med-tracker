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

      respond_to do |format|
        if @invitation.save
          InvitationMailer.with(invitation: @invitation).invite.deliver_later
          format.html { redirect_to admin_invitations_path, notice: 'Invitation sent' }
          format.turbo_stream do
            flash.now[:notice] = 'Invitation sent'
            render turbo_stream: [
              turbo_stream.replace('admin_invitations', Components::Admin::Invitations::IndexView.new),
              turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
            ]
          end
        else
          format.html do
            render Components::Admin::Invitations::IndexView.new(invitation: @invitation), status: :unprocessable_content
          end
          format.turbo_stream do
            render turbo_stream: turbo_stream.replace(
              'admin_invitations',
              Components::Admin::Invitations::IndexView.new(invitation: @invitation)
            ), status: :unprocessable_content
          end
        end
      end
    end

    private

    def invitation_params
      params.expect(invitation: %i[email role])
    end
  end
end
