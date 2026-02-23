class AddReorderStatusToMedicines < ActiveRecord::Migration[8.1]
  def change
    add_column :medicines, :reorder_status, :integer
  end
end
