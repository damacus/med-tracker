# frozen_string_literal: true

class AddClientUuidToMedicationTakes < ActiveRecord::Migration[8.1]
  def change
    add_column :medication_takes, :client_uuid, :string
    add_index :medication_takes, :client_uuid, unique: true, where: 'client_uuid IS NOT NULL'
  end
end
