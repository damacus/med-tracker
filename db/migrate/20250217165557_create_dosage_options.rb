class CreateDosageOptions < ActiveRecord::Migration[8.0]
  def change
    create_table :dosage_options do |t|
      t.references :medicine, null: false, foreign_key: true
      t.decimal :amount, precision: 10, scale: 2, null: false
      t.timestamps
    end
    add_index :dosage_options, [ :medicine_id, :amount ], unique: true
  end
end
