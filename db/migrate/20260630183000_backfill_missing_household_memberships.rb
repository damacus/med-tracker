# frozen_string_literal: true

class BackfillMissingHouseholdMemberships < ActiveRecord::Migration[8.1]
  def up
    return unless required_tables?

    create_missing_memberships
    promote_ownerless_households
    create_self_access_grants
    create_relationship_access_grants if table_exists?(:carer_relationships)
  end

  def down; end

  private

  def required_tables?
    %i[households household_memberships people person_access_grants].all? { |table_name| table_exists?(table_name) }
  end

  def create_missing_memberships
    execute <<~SQL.squish
      INSERT INTO household_memberships (
        household_id, account_id, person_id, role, status, permissions_version, joined_at, created_at, updated_at
      )
      SELECT people.household_id,
             people.account_id,
             people.id,
             'member',
             'active',
             1,
             CURRENT_TIMESTAMP,
             CURRENT_TIMESTAMP,
             CURRENT_TIMESTAMP
      FROM people
      WHERE people.household_id IS NOT NULL
        AND people.account_id IS NOT NULL
      ON CONFLICT (household_id, account_id) DO UPDATE
      SET person_id = EXCLUDED.person_id,
          status = 'active',
          revoked_at = NULL,
          updated_at = CURRENT_TIMESTAMP;
    SQL
  end

  def promote_ownerless_households
    execute <<~SQL.squish
      WITH owner_candidates AS (
        SELECT DISTINCT ON (household_memberships.household_id)
               household_memberships.id
        FROM household_memberships
        LEFT JOIN platform_admins
          ON platform_admins.account_id = household_memberships.account_id
         AND platform_admins.status = 'active'
        WHERE household_memberships.status = 'active'
          AND NOT EXISTS (
            SELECT 1
            FROM household_memberships active_owner
            WHERE active_owner.household_id = household_memberships.household_id
              AND active_owner.role = 'owner'
              AND active_owner.status = 'active'
          )
        ORDER BY household_memberships.household_id,
                 CASE WHEN platform_admins.id IS NULL THEN 1 ELSE 0 END,
                 household_memberships.account_id
      )
      UPDATE household_memberships
      SET role = 'owner',
          updated_at = CURRENT_TIMESTAMP
      WHERE id IN (SELECT id FROM owner_candidates);
    SQL
  end

  def create_self_access_grants
    execute <<~SQL.squish
      INSERT INTO person_access_grants (
        household_id, household_membership_id, person_id, access_level, relationship_type,
        granted_by_membership_id, created_at, updated_at
      )
      SELECT household_memberships.household_id,
             household_memberships.id,
             household_memberships.person_id,
             'manage',
             'self',
             household_memberships.id,
             CURRENT_TIMESTAMP,
             CURRENT_TIMESTAMP
      FROM household_memberships
      WHERE household_memberships.person_id IS NOT NULL
        AND household_memberships.status = 'active'
      ON CONFLICT (household_membership_id, person_id) WHERE revoked_at IS NULL DO UPDATE
      SET access_level = EXCLUDED.access_level,
          relationship_type = EXCLUDED.relationship_type,
          granted_by_membership_id = EXCLUDED.granted_by_membership_id,
          updated_at = CURRENT_TIMESTAMP;
    SQL
  end

  def create_relationship_access_grants
    execute <<~SQL.squish
      WITH owner_memberships AS (
        SELECT DISTINCT ON (household_id)
               id,
               household_id
        FROM household_memberships
        WHERE role = 'owner'
          AND status = 'active'
        ORDER BY household_id, id
      )
      INSERT INTO person_access_grants (
        household_id, household_membership_id, person_id, access_level, relationship_type,
        granted_by_membership_id, created_at, updated_at
      )
      SELECT household_memberships.household_id,
             household_memberships.id,
             carer_relationships.patient_id,
             CASE WHEN carer_relationships.relationship_type IN ('self', 'parent') THEN 'manage' ELSE 'record' END,
             CASE
               WHEN carer_relationships.relationship_type = 'professional_carer' THEN 'professional'
               WHEN carer_relationships.relationship_type IN ('parent', 'carer', 'family_member', 'self')
                 THEN carer_relationships.relationship_type
               ELSE 'family_member'
             END,
             owners.id,
             CURRENT_TIMESTAMP,
             CURRENT_TIMESTAMP
      FROM carer_relationships
      JOIN household_memberships
        ON household_memberships.person_id = carer_relationships.carer_id
       AND household_memberships.status = 'active'
      JOIN owner_memberships owners
        ON owners.household_id = household_memberships.household_id
      WHERE carer_relationships.active = true
      ON CONFLICT (household_membership_id, person_id) WHERE revoked_at IS NULL DO UPDATE
      SET access_level = EXCLUDED.access_level,
          relationship_type = EXCLUDED.relationship_type,
          granted_by_membership_id = EXCLUDED.granted_by_membership_id,
          updated_at = CURRENT_TIMESTAMP;
    SQL
  end
end
