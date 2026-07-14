# frozen_string_literal: true

module CareDelegation
  class Revoke
    class Error < StandardError; end
    class AmbiguousGrant < Error; end

    def initialize(relationship:, actor_membership: nil)
      @relationship = relationship
      @actor_membership = actor_membership
    end

    def call
      ActiveRecord::Base.transaction do
        relationship.lock!
        revoke_delegated_access! unless self_relationship?
        relationship.update!(active: false) if relationship.active?
        relationship
      end
    end

    private

    attr_reader :relationship, :actor_membership

    def self_relationship?
      relationship.carer_id == relationship.patient_id && relationship.relationship_type == 'self'
    end

    def revoke_delegated_access!
      raise_if_unowned_grant_authorizes!
      relationship.person_access_grants.where(revoked_at: nil).lock.find_each do |grant|
        access_change_for(grant).revoke_grant!(grant)
      end
    end

    def raise_if_unowned_grant_authorizes!
      membership = relationship.household.household_memberships.find_by(person: relationship.carer)
      return unless membership

      unowned_grants = membership.person_access_grants.active
      return unless unowned_grants.exists?(person: relationship.patient, carer_relationship: nil)

      raise AmbiguousGrant, 'an active unowned grant still authorizes this patient'
    end

    def access_change_for(grant)
      actor = actor_membership || grant.granted_by_membership || grant.household_membership
      Households::AccessChange.new(
        actor_account: actor&.account,
        actor_membership: actor,
        request: nil
      )
    end
  end
end
