# frozen_string_literal: true

# Preview all emails at http://localhost:3000/rails/mailers/invitation_mailer
class InvitationMailerPreview < ActionMailer::Preview
  # Preview this email at http://localhost:3000/rails/mailers/invitation_mailer/invite
  def invite
    invitation = Invitation.new(
      email: 'preview-invitee@example.com',
      role: :carer,
      token: 'preview-invitation-token-abc123',
      expires_at: 7.days.from_now
    )
    InvitationMailer.with(invitation: invitation).invite
  end
end
