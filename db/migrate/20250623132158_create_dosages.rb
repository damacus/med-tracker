class CreateDosages < ActiveRecord::Migration[8.0]
  def change
    create_table :dosages do |t|
      t.references :medicine, null: false, foreign_key: true
      t.decimal :amount
      t.string :unit
      t.string :frequency
      t.string :description

      t.timestamps
    end
  end
end
