class CreateMedicationTakes < ActiveRecord::Migration[8.0]
  def change
    create_table :medication_takes do |t|
      t.references :prescription, null: false, foreign_key: true
      t.datetime :taken_at
      t.text :notes

      t.timestamps
    end
  end
end
