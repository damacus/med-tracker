# frozen_string_literal: true

# Make foreign keys deferrable to allow fixture loading without disabling referential integrity.
#
# PostgreSQL foreign keys can be DEFERRABLE, meaning constraint checking is delayed until
# transaction commit. This allows Rails to load fixtures in any order within a transaction,
# then validate all constraints at the end.
#
# This is essential for CI environments where the database user may not have superuser
# privileges to disable referential integrity via SET session_replication_role = replica.
#
# See: https://www.postgresql.org/docs/current/sql-set-constraints.html
class MakeForeignKeysDeferrable < ActiveRecord::Migration[8.0]
  def up
    # Get all foreign keys and recreate them as deferrable
    foreign_keys_to_update.each do |fk|
      remove_foreign_key fk[:from_table], column: fk[:column]
      add_foreign_key fk[:from_table], fk[:to_table],
                      column: fk[:column],
                      deferrable: :deferred
    end
  end

  def down
    # Revert to immediate (non-deferrable) foreign keys
    foreign_keys_to_update.each do |fk|
      remove_foreign_key fk[:from_table], column: fk[:column]
      add_foreign_key fk[:from_table], fk[:to_table],
                      column: fk[:column]
    end
  end

  private

  def foreign_keys_to_update
    [
      # Core model relationships
      { from_table: :people, to_table: :accounts, column: :account_id },
      { from_table: :users, to_table: :people, column: :person_id },

      # Medicine relationships
      { from_table: :dosages, to_table: :medicines, column: :medicine_id },
      { from_table: :prescriptions, to_table: :people, column: :person_id },
      { from_table: :prescriptions, to_table: :medicines, column: :medicine_id },
      { from_table: :prescriptions, to_table: :dosages, column: :dosage_id },

      # Medication tracking
      { from_table: :medication_takes, to_table: :prescriptions, column: :prescription_id },
      { from_table: :medication_takes, to_table: :person_medicines, column: :person_medicine_id },
      { from_table: :person_medicines, to_table: :people, column: :person_id },
      { from_table: :person_medicines, to_table: :medicines, column: :medicine_id },

      # Carer relationships
      { from_table: :carer_relationships, to_table: :people, column: :carer_id },
      { from_table: :carer_relationships, to_table: :people, column: :patient_id }
    ]
  end
end
