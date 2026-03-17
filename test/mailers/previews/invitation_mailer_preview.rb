# frozen_string_literal: true

# Preview all emails at http://localhost:3000/rails/mailers/invitation_mailer
class InvitationMailerPreview < ActionMailer::Preview
  # Preview this email at http://localhost:3000/rails/mailers/invitation_mailer/invite
  def invite
    token = 'preview-invitation-token-abc123'
    invitation = Invitation.new(
      email: 'preview-invitee@example.com',
      role: :carer,
      expires_at: 7.days.from_now,
      token_digest: Invitation.digest(token)
    )
    invitation.instance_variable_set(:@plain_token, token)
    InvitationMailer.with(invitation: invitation, token: token).invite
  end
end
