# frozen_string_literal: true

class InvitationMailer < ApplicationMailer
  def invite
    assign_invitation_context
    deliver_invitation
  end

  private

  def assign_invitation_context
    @invitation = params[:invitation]
    @token = params[:token] || @invitation&.plain_token
    raise ArgumentError, 'Invitation token missing' if @token.blank?

    @role = @invitation.membership_role.humanize
  end

  def deliver_invitation
    mail(to: @invitation.email, subject: I18n.t('invitation_mailer.subject')) do |format|
      format.html do
        render body: render_mail_component(invitation_component), content_type: 'text/html'
      end
      format.text { render plain: invitation_text }
    end
  end

  def invitation_component
    Views::Mailers::Invitation.new(role: @role, accept_url: accept_invitation_url(token: @token))
  end

  def invitation_text
    [
      I18n.t('invitation_mailer.invite.title'),
      '',
      I18n.t('invitation_mailer.invite.invited_as', role: @role),
      '',
      "#{I18n.t('invitation_mailer.invite.accept_invitation_label')}:",
      accept_invitation_url(token: @token),
      '',
      I18n.t('invitation_mailer.invite.expiry_notice')
    ].join("\n")
  end
end
