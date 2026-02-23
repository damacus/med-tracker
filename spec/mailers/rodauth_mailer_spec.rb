# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RodauthMailer do
  fixtures :accounts

  describe '#verify_account' do
    let(:account) { accounts(:test) }
    let(:key) { 'abc123verifykey' }
    let(:mail) { described_class.verify_account(nil, account.id, key) }

    it 'sends to the account email address' do
      expect(mail.to).to eq([account.email])
    end

    it 'sends from the configured from address' do
      expect(mail.from).to eq(['noreply@medtracker.app'])
    end

    it 'has a subject containing "Verify"' do
      expect(mail.subject).to include('Verify')
    end

    it 'includes the verify account link in the body' do
      expect(mail.body.encoded).to include('verify-account')
    end

    it 'is a plain text email' do
      expect(mail.content_type).to include('text/plain')
    end
  end
end
