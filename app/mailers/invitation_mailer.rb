# frozen_string_literal: true

class InvitationMailer < ApplicationMailer
  def invite
    @invitation = params[:invitation]
    @token = @invitation.token

    body = <<~TEXT
      You have been invited to join MedTracker as a #{@invitation.role}.

      Accept your invitation:
      #{accept_invitation_url(token: @token)}
    TEXT

    mail(to: @invitation.email, subject: I18n.t('invitation_mailer.subject'), body: body, content_type: 'text/plain')
  end
end
