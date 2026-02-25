class AddOrderedAtToMedicines < ActiveRecord::Migration[8.1]
  def change
    add_column :medicines, :ordered_at, :datetime
    add_column :medicines, :reordered_at, :datetime
  end
end
