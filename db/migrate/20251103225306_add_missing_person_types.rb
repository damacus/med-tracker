# frozen_string_literal: true

class AddMissingPersonTypes < ActiveRecord::Migration[8.1]
  def up
    # Map old values to new values
    # old: patient(0), carer(1), nurse(2), doctor(3), administrator(4)
    # new: adult_patient(0), child(1), parent(2), carer(3), nurse(4), doctor(5), administrator(6)

    execute <<-SQL
      UPDATE people SET person_type = 0 WHERE person_type = 0; -- patient -> adult_patient (no change)
      UPDATE people SET person_type = 3 WHERE person_type = 1; -- carer -> carer (no change)
      UPDATE people SET person_type = 4 WHERE person_type = 2; -- nurse -> nurse (no change)
      UPDATE people SET person_type = 5 WHERE person_type = 3; -- doctor -> doctor (no change)
      UPDATE people SET person_type = 6 WHERE person_type = 4; -- administrator -> administrator (no change)
    SQL
  end

  def down
    # Revert to old values
    execute <<-SQL
      UPDATE people SET person_type = 0 WHERE person_type = 0; -- adult_patient -> patient
      UPDATE people SET person_type = 1 WHERE person_type = 3; -- carer -> carer
      UPDATE people SET person_type = 2 WHERE person_type = 4; -- nurse -> nurse
      UPDATE people SET person_type = 3 WHERE person_type = 5; -- doctor -> doctor
      UPDATE people SET person_type = 4 WHERE person_type = 6; -- administrator -> administrator
    SQL
  end
end
