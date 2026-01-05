# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Invitation do
  describe 'validations' do
    it { is_expected.to validate_presence_of(:email) }
    it { is_expected.to validate_presence_of(:role) }
    it { is_expected.to allow_value('test@example.com').for(:email) }
    it { is_expected.not_to allow_value('invalid-email').for(:email) }
  end

  describe 'enums' do
    subject(:invitation) { described_class.new }

    it 'defines role enum' do
      expect(invitation).to define_enum_for(:role)
        .with_values(administrator: 0, doctor: 1, nurse: 2, carer: 3, parent: 4, minor: 5)
    end
  end

  describe 'callbacks' do
    it 'generates a token before creation' do
      invitation = create(:invitation)
      expect(invitation.token).to be_present
    end

    it 'sets expiration date before creation' do
      invitation = create(:invitation)
      expect(invitation.expires_at).to be_present
      expect(invitation.expires_at).to be > Time.current
    end
  end

  describe 'scopes' do
    describe '.pending' do
      let!(:pending_invitation) { create(:invitation) }
      let!(:accepted_invitation) { create(:invitation, :accepted) }
      let!(:expired_invitation) { create(:invitation, :expired) }

      it 'returns only pending invitations' do
        expect(described_class.pending).to include(pending_invitation)
        expect(described_class.pending).not_to include(accepted_invitation)
        expect(described_class.pending).not_to include(expired_invitation)
      end
    end
  end

  describe '#expired?' do
    it 'returns true if expires_at is in the past' do
      invitation = build(:invitation, :expired)
      expect(invitation).to be_expired
    end

    it 'returns false if expires_at is in the future' do
      invitation = build(:invitation, expires_at: 1.day.from_now)
      expect(invitation).not_to be_expired
    end
  end

  describe '#accepted?' do
    it 'returns true if accepted_at is present' do
      invitation = build(:invitation, :accepted)
      expect(invitation).to be_accepted
    end

    it 'returns false if accepted_at is nil' do
      invitation = build(:invitation, accepted_at: nil)
      expect(invitation).not_to be_accepted
    end
  end
end
