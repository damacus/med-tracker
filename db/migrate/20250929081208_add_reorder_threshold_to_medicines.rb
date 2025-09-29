# frozen_string_literal: true

# rubocop:disable Style/Documentation

class AddReorderThresholdToMedicines < ActiveRecord::Migration[8.0]
  def change
    add_column :medicines, :reorder_threshold, :integer, default: 10, null: false
  end
end
# rubocop:enable Style/Documentation
