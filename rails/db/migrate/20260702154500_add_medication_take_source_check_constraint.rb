# frozen_string_literal: true

class AddMedicationTakeSourceCheckConstraint < ActiveRecord::Migration[8.1]
  def change
    add_check_constraint :medication_takes,
                         'num_nonnulls(schedule_id, person_medication_id) = 1',
                         name: 'chk_medication_takes_exactly_one_source'
  end
end
