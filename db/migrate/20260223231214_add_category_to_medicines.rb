# frozen_string_literal: true

class AddCategoryToMedicines < ActiveRecord::Migration[8.1]
  def change
    add_column :medicines, :category, :string
  end
end
