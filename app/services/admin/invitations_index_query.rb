# frozen_string_literal: true

module Admin
  class InvitationsIndexQuery
    Result = Data.define(:invitations, :resendable_invitation_ids)

    attr_reader :scope

    def initialize(scope:)
      @scope = scope
    end

    def call
      current_invitations = scope.order(created_at: :desc)

      Result.new(
        invitations: current_invitations,
        resendable_invitation_ids: resendable_invitation_ids(current_invitations)
      )
    end

    private

    def resendable_invitation_ids(current_invitations)
      pending_counts_by_email = scope.pending.group(:email).count

      current_invitations.filter_map do |invitation|
        next if invitation.accepted?

        pending_count = pending_counts_by_email[invitation.email].to_i
        next invitation.id if invitation.pending? && pending_count == 1
        next invitation.id if invitation.expired? && pending_count.zero?
      end
    end
  end
end
