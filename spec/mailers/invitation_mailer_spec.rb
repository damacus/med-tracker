# frozen_string_literal: true

require 'rails_helper'

RSpec.describe InvitationMailer do
  describe '#invite' do
    let(:token) { 'test-token-123' }
    let(:invitation) do
      build(:household_invitation, email: 'invitee@example.com', membership_role: :member,
                                   token_digest: HouseholdInvitation.digest(token))
    end
    let(:mail) { described_class.with(invitation: invitation, token: token).invite }

    it 'renders the recipient email' do
      expect(mail.to).to eq(['invitee@example.com'])
    end

    it 'renders the subject' do
      expect(mail.subject).to eq(I18n.t('invitation_mailer.subject'))
    end

    it 'renders the sender email' do
      expect(mail.from).to eq(['noreply@medtracker.app'])
    end

    it 'includes the invitation acceptance URL in the body' do
      expect(mail.body.encoded).to include('invitations/accept?token=test-token-123')
    end

    it 'includes the role in the body' do
      expect(mail.body.encoded).to include('Member')
    end

    it 'is a multipart email with HTML and plain text parts' do
      expect(mail.content_type).to include('multipart/alternative')
    end

    it 'renders branded HTML without ERB templates' do
      html = mail.html_part.body.encoded

      expect(html).to include('MedTracker')
      expect(html).to include('style=')
      expect(html).to include('invitations/accept?token=test-token-123')
      expect(Rails.root.glob('app/views/invitation_mailer/*.erb')).to be_empty
      expect(Rails.root.glob('app/views/layouts/mailer*.erb')).to be_empty
    end
  end
end
