# frozen_string_literal: true

module PortableData
  class ImportedPersonAccessGranter
    def initialize(household:, membership:)
      @household = household
      @membership = membership
    end

    def call(person)
      return if membership.blank?

      household.with_lock do
        grant = active_grant_for(person)
        next if grant.carer_relationship && preserve_relationship_grant!(grant)

        Households::AccessChange.for(membership).upsert_grant!(grant, grant_attributes(grant, person))
      end
    end

    private

    attr_reader :household, :membership

    def active_grant_for(person)
      household.person_access_grants
               .where(revoked_at: nil)
               .find_or_initialize_by(household_membership: membership, person: person)
    end

    def preserve_relationship_grant!(grant)
      return true if grant.manage? && grant.expires_at.nil?

      raise Importer::Error, 'relationship-owned access grant conflicts with imported manage access'
    end

    def grant_attributes(grant, person)
      {
        household: household,
        household_membership: membership,
        person: person,
        access_level: :manage,
        relationship_type: grant.relationship_type || :family_member,
        granted_by_membership: grant.granted_by_membership || membership
      }
    end
  end
end
