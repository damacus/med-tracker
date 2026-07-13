# frozen_string_literal: true

module CareDelegation
  class Assign
    class Error < StandardError; end
    class GrantConflict < Error; end
    class InvalidAccessLevel < Error; end
    class InvalidRelationshipType < Error; end

    GRANT_ATTRIBUTES = {
      'family_member' => %i[manage family_member],
      'parent' => %i[manage parent],
      'professional_carer' => %i[record professional],
      'self' => %i[manage self]
    }.freeze

    def initialize(carer:, patient:, relationship_type:, **options)
      @carer = carer
      @patient = patient
      @relationship_type = relationship_type.to_s
      @access_level = options[:access_level]&.to_s
      @granted_by_membership = options[:granted_by_membership]
      @expires_at = options[:expires_at]
    end

    def call
      ActiveRecord::Base.transaction(requires_new: true) do
        access_level, grant_relationship_type = grant_attributes
        patient.household.lock!
        relationship = persist_relationship!
        membership = persist_membership!(relationship.household)
        if membership
          persist_self_grant!(membership)
          persist_patient_grant!(relationship, membership, access_level, grant_relationship_type) unless
            self_relationship?(relationship)
        end
        relationship
      end
    end

    private

    attr_reader :carer, :patient, :relationship_type, :access_level, :granted_by_membership, :expires_at

    def persist_relationship!
      relationship = CarerRelationship.find_or_initialize_by(
        household: patient&.household,
        carer: carer,
        patient: patient
      )
      relationship.relationship_type = relationship_type
      relationship.active = true
      relationship.save! if relationship.new_record? || relationship.changed?
      relationship
    end

    def persist_membership!(household)
      account = carer&.account
      return unless account

      membership = household.household_memberships.find_or_initialize_by(account: account)
      persist_membership_record!(membership)
      membership
    end

    def persist_membership_record!(membership)
      membership.person = carer
      membership.role = :member if membership.role.blank?
      membership.status = :active
      membership.save! if membership.new_record? || membership.changed?
    end

    def persist_self_grant!(membership)
      grant = self_grant(membership)
      grant.assign_attributes(access_level: :manage, relationship_type: :self, expires_at: nil, revoked_at: nil,
                              carer_relationship: nil)
      grant.granted_by_membership ||= granted_by_membership || membership
      persist_changed!(grant)
    end

    def persist_patient_grant!(relationship, membership, required_access_level, grant_relationship_type)
      grant = available_patient_grant(membership)
      return preserve_manual_grant!(grant, required_access_level) if manual_grant?(grant, relationship)

      grant ||= owned_patient_grant(relationship, membership)
      assign_patient_grant(grant, membership, required_access_level, grant_relationship_type)
      persist_changed!(grant)
    end

    def available_patient_grant(membership)
      grant = unrevoked_grant(membership, patient)
      return grant unless grant_expired?(grant)

      retire_expired_grant!(grant)
      nil
    end

    def manual_grant?(grant, relationship)
      grant && grant.carer_relationship_id != relationship.id
    end

    def self_relationship?(relationship)
      relationship.carer_id == relationship.patient_id && relationship.relationship_type == 'self'
    end

    def owned_patient_grant(relationship, membership)
      relationship.person_access_grants
                  .where(household_membership: membership, person: patient)
                  .order(id: :desc).first ||
        relationship.person_access_grants.build(
          household: relationship.household,
          household_membership: membership,
          person: patient
        )
    end

    def assign_patient_grant(grant, membership, required_access_level, grant_relationship_type)
      grant.assign_attributes(
        access_level: required_access_level,
        relationship_type: grant_relationship_type,
        granted_by_membership: granted_by_membership || membership,
        expires_at: expires_at,
        revoked_at: nil
      )
    end

    def self_grant(membership)
      active_grant(membership, carer) || latest_grant(membership, carer) ||
        membership.person_access_grants.build(household: membership.household, person: carer)
    end

    def persist_changed!(grant)
      grant.save! if grant.new_record? || grant.changed?
    end

    def preserve_manual_grant!(grant, required_access_level)
      return if grant.cover_access?(required_access_level) && grant.cover_expiry?(expires_at)

      raise GrantConflict, 'existing manual grant does not cover the delegated access level'
    end

    def active_grant(membership, grant_person)
      membership.person_access_grants.active.lock.find_by(person: grant_person)
    end

    def unrevoked_grant(membership, grant_person)
      membership.person_access_grants.where(person: grant_person, revoked_at: nil).lock.first
    end

    def retire_expired_grant!(grant)
      grant.update!(revoked_at: Time.current)
    end

    def grant_expired?(grant)
      grant&.expires_at.present? && grant.expires_at <= Time.current
    end

    def latest_grant(membership, grant_person)
      membership.person_access_grants.where(person: grant_person).order(id: :desc).first
    end

    def grant_attributes
      default_access_level, grant_relationship_type = GRANT_ATTRIBUTES.fetch(relationship_type) do
        raise InvalidRelationshipType, "unsupported carer relationship type: #{relationship_type}"
      end
      [resolved_access_level(default_access_level), grant_relationship_type]
    end

    def resolved_access_level(default_access_level)
      return default_access_level if access_level.blank?
      return access_level if PersonAccessGrant.access_levels.key?(access_level)

      raise InvalidAccessLevel, "unsupported access level: #{access_level}"
    end
  end
end
