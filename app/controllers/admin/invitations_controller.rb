# frozen_string_literal: true

module Admin
  class InvitationsController < ApplicationController
    def index
      authorize :invitation, :index?

      render Components::Admin::Invitations::IndexView.new(invitations: invitations)
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
            render_index_turbo
          end
        else
          format.html do
            render Components::Admin::Invitations::IndexView.new(invitation: @invitation, invitations: invitations),
                   status: :unprocessable_content
          end
          format.turbo_stream do
            render turbo_stream: turbo_stream.replace(
              'admin_invitations',
              Components::Admin::Invitations::IndexView.new(invitation: @invitation, invitations: invitations)
            ), status: :unprocessable_content
          end
        end
      end
    end

    def resend
      @invitation = Invitation.find(params[:id])
      authorize @invitation, :resend?

      if @invitation.resendable?
        @invitation.resend!
        InvitationMailer.with(invitation: @invitation).invite.deliver_later
        redirect_with_invitation_notice('Invitation resent')
      else
        redirect_with_invitation_alert('Accepted invitations cannot be resent')
      end
    end

    private

    def invitation_params
      params.expect(invitation: %i[email role])
    end

    def invitations
      Invitation.order(created_at: :desc)
    end

    def render_index_turbo
      render turbo_stream: [
        turbo_stream.replace('admin_invitations', Components::Admin::Invitations::IndexView.new(invitations: invitations)),
        turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
      ]
    end

    def redirect_with_invitation_notice(message)
      respond_to do |format|
        format.html { redirect_to admin_invitations_path, notice: message }
        format.turbo_stream do
          flash.now[:notice] = message
          render_index_turbo
        end
      end
    end

    def redirect_with_invitation_alert(message)
      respond_to do |format|
        format.html { redirect_to admin_invitations_path, alert: message }
        format.turbo_stream do
          flash.now[:alert] = message
          render_index_turbo
        end
      end
    end
  end
end
