class RemoveStandardDosageFromMedicines < ActiveRecord::Migration[8.0]
  def up
    remove_column :medicines, :standard_dosage
  end

  def down
    add_column :medicines, :standard_dosage, :string
  end
end
