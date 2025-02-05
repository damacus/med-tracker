class CreateMedicines < ActiveRecord::Migration[8.0]
  def change
    create_table :medicines do |t|
      t.string :name
      t.text :description
      t.string :standard_dosage
      t.text :warnings

      t.timestamps
    end
  end
end
