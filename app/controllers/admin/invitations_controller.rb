# frozen_string_literal: true

module Admin
  class InvitationsController < ApplicationController
    def index
      authorize :invitation, :index?

      render invitation_index_view
    end

    def create
      authorize :invitation, :create?

      @invitation = Invitation.new(invitation_params)

      respond_to do |format|
        if @invitation.save
          InvitationMailer.with(invitation: @invitation).invite.deliver_later
          format.html { redirect_to admin_invitations_path, notice: t('admin.invitations.created') }
          format.turbo_stream do
            flash.now[:notice] = t('admin.invitations.created')
            render_index_turbo
          end
        else
          format.html do
            render invitation_index_view(invitation: @invitation), status: :unprocessable_content
          end
          format.turbo_stream do
            render turbo_stream: turbo_stream.replace(
              'admin_invitations',
              invitation_index_view(invitation: @invitation)
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
        redirect_with_invitation_notice(t('admin.invitations.resent'))
      elsif @invitation.accepted?
        redirect_with_invitation_alert(t('admin.invitations.cannot_resend_accepted'))
      else
        redirect_with_invitation_alert(t('admin.invitations.resend_failed'))
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
        turbo_stream.replace('admin_invitations', invitation_index_view),
        turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
      ]
    end

    def invitation_index_view(invitation: Invitation.new)
      current_invitations = invitations
      Components::Admin::Invitations::IndexView.new(
        invitation: invitation,
        invitations: current_invitations,
        resendable_invitation_ids: resendable_invitation_ids(current_invitations)
      )
    end

    def resendable_invitation_ids(current_invitations)
      pending_counts_by_email = Invitation.pending.group(:email).count

      current_invitations.filter_map do |invitation|
        next if invitation.accepted?

        pending_count = pending_counts_by_email[invitation.email].to_i
        next invitation.id if invitation.pending? && pending_count == 1
        next invitation.id if invitation.expired? && pending_count.zero?
      end
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
