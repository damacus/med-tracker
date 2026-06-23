# frozen_string_literal: true

class InvitationMailer < ApplicationMailer
  def invite
    @invitation = params[:invitation]
    @token = params[:token] || @invitation&.plain_token
    raise ArgumentError, 'Invitation token missing' if @token.blank?

    @role = @invitation.membership_role.humanize

    mail(to: @invitation.email, subject: I18n.t('invitation_mailer.subject'))
  end
end
