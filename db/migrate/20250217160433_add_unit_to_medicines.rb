class AddUnitToMedicines < ActiveRecord::Migration[8.0]
  def change
    add_column :medicines, :unit, :string, null: false, default: 'tablet'
  end
end
