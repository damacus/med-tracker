class CreateRecommendedDosages < ActiveRecord::Migration[8.0]
  def change
    create_table :recommended_dosages do |t|
      t.references :medicine, null: false, foreign_key: true
      t.integer :min_age
      t.integer :max_age
      t.decimal :amount_ml
      t.integer :frequency_per_day

      t.timestamps
    end
  end
end
