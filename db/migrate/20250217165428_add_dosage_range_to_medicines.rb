class AddDosageRangeToMedicines < ActiveRecord::Migration[8.0]
  def change
    add_column :medicines, :min_dosage, :decimal, precision: 10, scale: 2
    add_column :medicines, :max_dosage, :decimal, precision: 10, scale: 2
    rename_column :medicines, :standard_dosage, :dosage
  end
end
