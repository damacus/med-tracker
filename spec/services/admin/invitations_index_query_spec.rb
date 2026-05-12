# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Admin::InvitationsIndexQuery do
  describe '#call' do
    it 'returns invitations newest first with the resendable and cancellable ids' do
      newest_pending = create(:invitation, email: 'solo.pending@example.com', created_at: 1.hour.from_now)
      expired_duplicate = create(:invitation, :expired, email: 'duplicate@example.com')
      pending_duplicate = create(:invitation, email: 'duplicate@example.com')
      expired_singleton = create(:invitation, :expired, email: 'solo.expired@example.com')
      accepted = create(:invitation, :accepted, email: 'accepted@example.com')

      result = described_class.new(scope: Invitation.all).call

      expect(result.invitations.first).to eq(newest_pending)
      expect(result.resendable_invitation_ids).to include(newest_pending.id, pending_duplicate.id, expired_singleton.id)
      expect(result.resendable_invitation_ids).not_to include(expired_duplicate.id, accepted.id)
      expect(result.cancellable_invitation_ids).to include(
        newest_pending.id, expired_duplicate.id, pending_duplicate.id, expired_singleton.id
      )
      expect(result.cancellable_invitation_ids).not_to include(accepted.id)
    end
  end
end
