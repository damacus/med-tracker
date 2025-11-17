# frozen_string_literal: true

class SimplifyPersonTypes < ActiveRecord::Migration[8.0]
  def up
    # Map old person types to new simplified types
    # Old: adult_patient(0), child(1), parent(2), carer(3), nurse(4), doctor(5), administrator(6)
    # New: adult(0), minor(1), dependent_adult(2)

    # Map all adults (parent, carer, nurse, doctor, administrator, adult_patient) to adult(0)
    execute <<-SQL.squish
      UPDATE people#{' '}
      SET person_type = 0#{' '}
      WHERE person_type IN (0, 2, 3, 4, 5, 6);
    SQL

    # Map child(1) to minor(1) - no change needed

    # NOTE: We don't have dependent_adult data yet, but the enum value is reserved
  end

  def down
    # Cannot reliably revert as we lose information about who was a doctor, nurse, etc.
    # Those distinctions now live in the User role
    raise ActiveRecord::IrreversibleMigration,
          'Cannot revert person_type simplification - role information moved to User model'
  end
end
