# frozen_string_literal: true

class AddHouseholdCompositeForeignKeys < ActiveRecord::Migration[8.1]
  def change
    add_candidate_keys
    add_composite_foreign_key :household_memberships, :people, %i[person_id household_id]
    add_composite_foreign_key :person_access_grants, :household_memberships, %i[household_membership_id household_id]
    add_composite_foreign_key :person_access_grants, :people, %i[person_id household_id]
    add_composite_foreign_key :person_access_grants, :household_memberships, %i[granted_by_membership_id household_id]
    add_composite_foreign_key :location_memberships, :people, %i[person_id household_id]
    add_composite_foreign_key :location_memberships, :locations, %i[location_id household_id]
    add_composite_foreign_key :medications, :locations, %i[location_id household_id]
    add_composite_foreign_key :dosages, :medications, %i[medication_id household_id]
    add_composite_foreign_key :schedules, :people, %i[person_id household_id]
    add_composite_foreign_key :schedules, :medications, %i[medication_id household_id]
    add_composite_foreign_key :schedules, :dosages, %i[source_dosage_option_id household_id]
    add_composite_foreign_key :person_medications, :people, %i[person_id household_id]
    add_composite_foreign_key :person_medications, :medications, %i[medication_id household_id]
    add_composite_foreign_key :person_medications, :dosages, %i[source_dosage_option_id household_id]
    add_composite_foreign_key :medication_takes, :schedules, %i[schedule_id household_id]
    add_composite_foreign_key :medication_takes, :person_medications, %i[person_medication_id household_id]
    add_composite_foreign_key :medication_takes, :medications, %i[taken_from_medication_id household_id]
    add_composite_foreign_key :medication_takes, :locations, %i[taken_from_location_id household_id]
    add_composite_foreign_key :notification_preferences, :people, %i[person_id household_id]
  end

  private

  def add_candidate_keys
    %i[
      people
      locations
      location_memberships
      medications
      dosages
      schedules
      person_medications
      medication_takes
      notification_preferences
      household_memberships
      person_access_grants
      household_invitations
      security_audit_events
    ].each do |table_name|
      next unless column_exists?(table_name, :household_id)

      remove_index table_name, column: %i[id household_id], if_exists: true
      add_index table_name, %i[id household_id], unique: true
    end
  end

  def add_composite_foreign_key(from_table, to_table, columns)
    add_foreign_key from_table,
                    to_table,
                    column: columns,
                    primary_key: %i[id household_id],
                    validate: false,
                    name: "fk_#{from_table}_#{columns.first}_household"
  end
end
