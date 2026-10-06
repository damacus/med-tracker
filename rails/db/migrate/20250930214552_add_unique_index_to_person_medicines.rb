# frozen_string_literal: true

class AddUniqueIndexToPersonMedicines < ActiveRecord::Migration[8.0]
  def change
    add_index :person_medicines, %i[person_id medicine_id], unique: true
  end
end
