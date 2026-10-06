# frozen_string_literal: true

module Admin
  class AmbiguousPersonAccessGrantsIndexQuery
    HOUSEHOLD_MEMBERSHIP_JOIN = <<~SQL.squish.freeze
      INNER JOIN household_memberships
        ON household_memberships.id = person_access_grants.household_membership_id
       AND household_memberships.household_id = person_access_grants.household_id
    SQL
    COMPATIBLE_RELATIONSHIP_JOIN = <<~SQL.squish.freeze
      INNER JOIN (
        SELECT DISTINCT ON (household_id, carer_id, patient_id)
          id,
          household_id,
          carer_id,
          patient_id,
          relationship_type,
          active
        FROM carer_relationships
        WHERE active = TRUE
        ORDER BY household_id, carer_id, patient_id, id DESC
      ) AS compatible_carer_relationships
        ON compatible_carer_relationships.household_id = person_access_grants.household_id
       AND compatible_carer_relationships.carer_id = household_memberships.person_id
       AND compatible_carer_relationships.patient_id = person_access_grants.person_id
    SQL
    SELECT_COLUMNS = <<~SQL.squish.freeze
      person_access_grants.id,
      person_access_grants.household_id,
      person_access_grants.household_membership_id,
      person_access_grants.person_id,
      person_access_grants.carer_relationship_id,
      person_access_grants.access_level,
      person_access_grants.relationship_type,
      person_access_grants.expires_at,
      person_access_grants.revoked_at,
      person_access_grants.created_at,
      person_access_grants.updated_at,
      compatible_carer_relationships.id AS compatible_relationship_id,
      compatible_carer_relationships.relationship_type AS compatible_relationship_type,
      compatible_carer_relationships.active AS compatible_relationship_active
    SQL

    attr_reader :scope

    def initialize(scope:)
      @scope = scope
    end

    def call
      scope
        .merge(PersonAccessGrant.active)
        .where(carer_relationship_id: nil)
        .joins(household_membership_join)
        .joins(compatible_relationship_join)
        .includes(:person, household_membership: :person)
        .select(select_columns)
        .distinct
        .order(created_at: :desc, id: :desc)
    end

    private

    def household_membership_join
      HOUSEHOLD_MEMBERSHIP_JOIN
    end

    def compatible_relationship_join
      COMPATIBLE_RELATIONSHIP_JOIN
    end

    def select_columns
      SELECT_COLUMNS
    end
  end
end
