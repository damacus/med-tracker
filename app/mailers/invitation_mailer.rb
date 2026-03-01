# frozen_string_literal: true

class InvitationMailer < ApplicationMailer
  def invite
    @invitation = params[:invitation]
    @token = @invitation.token
    @role = I18n.t("activerecord.attributes.invitation.roles.#{@invitation.role}", default: @invitation.role.humanize)

    mail(to: @invitation.email, subject: I18n.t('invitation_mailer.subject'))
  end
end
