class CreatePrescriptions < ActiveRecord::Migration[8.0]
  def change
    create_table :prescriptions do |t|
      t.references :person, null: false, foreign_key: true
      t.references :medicine, null: false, foreign_key: true
      t.string :dosage
      t.string :frequency
      t.date :start_date
      t.date :end_date
      t.text :notes

      t.timestamps
    end
  end
end
