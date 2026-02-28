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

    it 'is a multipart email with HTML and plain text parts' do
      expect(mail.content_type).to include('multipart/alternative')
    end
  end

  describe '#reset_password' do
    let(:account) { accounts(:test) }
    let(:key) { 'abc123resetkey' }
    let(:mail) { described_class.reset_password(nil, account.id, key) }

    it 'sends to the account email address' do
      expect(mail.to).to eq([account.email])
    end

    it 'sends from the configured from address' do
      expect(mail.from).to eq(['noreply@medtracker.app'])
    end

    it 'has a subject containing "Reset"' do
      expect(mail.subject).to include('Reset')
    end

    it 'includes the reset password link in the body' do
      expect(mail.body.encoded).to include('reset-password')
    end

    it 'is a multipart email with HTML and plain text parts' do
      expect(mail.content_type).to include('multipart/alternative')
    end
  end

  describe '#verify_login_change' do
    let(:account) { accounts(:test) }
    let(:key) { 'abc123loginchangekey' }
    let(:mail) { described_class.verify_login_change(nil, account.id, key) }

    it 'sends to the account email address' do
      expect(mail.to).to eq([account.email])
    end

    it 'sends from the configured from address' do
      expect(mail.from).to eq(['noreply@medtracker.app'])
    end

    it 'has a subject containing "login"' do
      expect(mail.subject).to include('login')
    end

    it 'includes the verify login change link in the body' do
      expect(mail.body.encoded).to include('verify-login-change')
    end

    it 'is a multipart email with HTML and plain text parts' do
      expect(mail.content_type).to include('multipart/alternative')
    end
  end

  describe '#unlock_account' do
    let(:account) { accounts(:test) }
    let(:key) { 'abc123unlockkey' }
    let(:mail) { described_class.unlock_account(nil, account.id, key) }

    it 'sends to the account email address' do
      expect(mail.to).to eq([account.email])
    end

    it 'sends from the configured from address' do
      expect(mail.from).to eq(['noreply@medtracker.app'])
    end

    it 'has a subject containing "Unlock"' do
      expect(mail.subject).to include('Unlock')
    end

    it 'includes the unlock account link in the body' do
      expect(mail.body.encoded).to include('unlock-account')
    end

    it 'is a multipart email with HTML and plain text parts' do
      expect(mail.content_type).to include('multipart/alternative')
    end
  end
end
