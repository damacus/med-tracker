class CreateMedicineDosages < ActiveRecord::Migration[8.0]
  def change
    create_table :medicine_dosages do |t|
      t.decimal :amount
      t.string :unit
      t.string :description
      t.boolean :is_default
      t.references :medicine, null: false, foreign_key: true

      t.timestamps
    end
  end
end
