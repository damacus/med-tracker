# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Admin::InvitationsIndexQuery do
  describe '#call' do
    it 'returns invitations newest first with the resendable and cancellable ids' do
      household = create(:household_invitation).household
      inviter = household.household_memberships.owner.sole
      invitations = create_invitations(household, inviter)

      result = described_class.new(scope: household.household_invitations).call

      expect(result.invitations.first).to eq(invitations.fetch(:newest_pending))
      expect(result.resendable_invitation_ids).to include(invitations.fetch(:newest_pending).id,
                                                          invitations.fetch(:expired_singleton).id)
      expect(result.resendable_invitation_ids).not_to include(invitations.fetch(:accepted).id,
                                                              invitations.fetch(:revoked).id)
      expect(result.cancellable_invitation_ids).to include(invitations.fetch(:newest_pending).id,
                                                           invitations.fetch(:expired_singleton).id)
      expect(result.cancellable_invitation_ids).not_to include(invitations.fetch(:accepted).id,
                                                               invitations.fetch(:revoked).id)
    end
  end

  def create_invitations(household, inviter)
    {
      newest_pending: create(:household_invitation, household: household, invited_by_membership: inviter,
                                                    email: 'solo.pending@example.com', created_at: 1.hour.from_now),
      expired_singleton: create(:household_invitation, :expired, household: household,
                                                                 invited_by_membership: inviter,
                                                                 email: 'solo.expired@example.com'),
      accepted: create(:household_invitation, :accepted, household: household, invited_by_membership: inviter,
                                                         email: 'accepted@example.com'),
      revoked: create(:household_invitation, household: household, invited_by_membership: inviter,
                                             email: 'revoked@example.com', revoked_at: Time.current)
    }
  end
end
