class AddDoseCycleToPersonMedications < ActiveRecord::Migration[8.1]
  def change
    add_column :person_medications, :dose_cycle, :integer
  end
end
