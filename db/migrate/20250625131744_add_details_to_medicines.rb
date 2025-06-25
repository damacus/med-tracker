class AddDetailsToMedicines < ActiveRecord::Migration[8.0]
  def change
    add_column :medicines, :dosage_amount, :float
    add_column :medicines, :dosage_unit, :string
    add_column :medicines, :stock, :integer
    add_column :medicines, :expiry_date, :date
    add_column :medicines, :description, :text
    add_column :medicines, :warnings, :text
  end
end
