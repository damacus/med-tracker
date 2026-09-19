# frozen_string_literal: true

module Admin
  class AmbiguousPersonAccessGrantClassifier
    class Error < StandardError; end
    class InvalidDecision < Error; end
    class StaleDecision < Error; end

    DECISIONS = %w[attach manual revoke].freeze
    COMPATIBLE_GRANT_TYPES = {
      'parent' => 'parent',
      'family_member' => 'family_member',
      'professional_carer' => 'professional',
      'self' => 'self'
    }.freeze

    def initialize(grant:, actor_membership:, request: nil)
      @grant = grant
      @actor_membership = actor_membership
      @request = request
    end

    def call(decision:, reason:, carer_relationship_id: nil)
      @decision = decision.to_s
      @reason = reason.to_s.strip
      @carer_relationship_id = carer_relationship_id
      validate_decision!
      ActiveRecord::Base.transaction do
        grant.lock!
        raise_if_stale_grant!
        apply_decision!
      end
      grant
    end

    private

    attr_reader :grant, :decision, :actor_membership, :reason, :carer_relationship_id, :request

    def validate_decision!
      raise InvalidDecision, "unsupported decision: #{decision}" unless DECISIONS.include?(decision)
      raise InvalidDecision, 'a reason is required' if reason.blank?
    end

    def raise_if_stale_grant!
      stale_reason = stale_grant_reason
      raise StaleDecision, stale_reason if stale_reason
    end

    def stale_grant_reason
      return 'grant is already resolved' if grant.disposition.present?
      return 'grant is already owned by a relationship' if grant.carer_relationship_id.present?
      return 'grant is already revoked' if grant.revoked_at.present?
      return 'grant has expired' if grant.expires_at.present? && grant.expires_at <= Time.current

      nil
    end

    def apply_decision!
      case decision
      when 'attach' then apply_attach!
      when 'manual' then apply_manual!
      when 'revoke' then apply_revoke!
      end
    end

    def apply_attach!
      relationship = CarerRelationship.lock.find_by(id: carer_relationship_id)
      raise StaleDecision, 'no compatible active relationship' unless compatible_relationship?(relationship)

      access_change.update_grant!(grant, classification_attributes.merge(carer_relationship: relationship))
    end

    def apply_manual!
      access_change.update_grant!(grant, classification_attributes.merge(disposition: 'manual'))
    end

    def apply_revoke!
      access_change.update_grant!(grant, classification_attributes.merge(revoked_at: Time.current))
    end

    def compatible_relationship?(relationship)
      return false unless relationship&.active?
      return false unless relationship.household_id == grant.household_id
      return false unless same_pair?(relationship)

      COMPATIBLE_GRANT_TYPES[relationship.relationship_type] == grant.relationship_type
    end

    def same_pair?(relationship)
      relationship.carer_id == grant.household_membership&.person_id &&
        relationship.patient_id == grant.person_id
    end

    def classification_attributes
      {
        classified_at: Time.current,
        classified_by_membership: actor_membership,
        classification_reason: reason
      }
    end

    def access_change
      Households::AccessChange.new(
        actor_account: actor_membership&.account,
        actor_membership: actor_membership,
        request: request
      )
    end
  end
end
