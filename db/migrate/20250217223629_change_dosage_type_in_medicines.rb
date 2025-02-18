class ChangeDosageTypeInMedicines < ActiveRecord::Migration[8.0]
  def up
    change_column :medicines, :dosage, :decimal, precision: 10, scale: 2
  end

  def down
    change_column :medicines, :dosage, :string
  end
end
