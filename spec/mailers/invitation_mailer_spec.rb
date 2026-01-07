# frozen_string_literal: true

require 'rails_helper'

RSpec.describe InvitationMailer do
  describe '#invite' do
    let(:invitation) { Invitation.new(email: 'invitee@example.com', role: :carer, token: 'test-token-123') }
    let(:mail) { described_class.with(invitation: invitation).invite }

    it 'renders the recipient email' do
      expect(mail.to).to eq(['invitee@example.com'])
    end

    it 'renders the subject' do
      expect(mail.subject).to eq(I18n.t('invitation_mailer.subject'))
    end

    it 'renders the sender email' do
      expect(mail.from).to eq(['from@example.com'])
    end

    it 'includes the invitation acceptance URL in the body' do
      expect(mail.body.encoded).to include('invitations/accept?token=test-token-123')
    end

    it 'includes the role in the body' do
      expect(mail.body.encoded).to include('carer')
    end

    it 'is a plain text email' do
      expect(mail.content_type).to include('text/plain')
    end
  end
end
