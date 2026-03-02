class AddDefaultsToDosages < ActiveRecord::Migration[8.1]
  def change
    add_column :dosages, :default_for_adults,              :boolean, default: false, null: false
    add_column :dosages, :default_for_children,            :boolean, default: false, null: false
    add_column :dosages, :default_max_daily_doses,         :integer
    add_column :dosages, :default_min_hours_between_doses, :decimal, precision: 4, scale: 1
    add_column :dosages, :default_dose_cycle,              :integer

    add_index :dosages, :medication_id, unique: true,
              where: 'default_for_adults = true',  name: 'index_dosages_one_adult_default'
    add_index :dosages, :medication_id, unique: true,
              where: 'default_for_children = true', name: 'index_dosages_one_child_default'
  end
end
