# frozen_string_literal: true

class AddFriendlyNameToMedications < ActiveRecord::Migration[8.1]
  def change
    return if column_exists?(:medications, :friendly_name)

    add_column :medications, :friendly_name, :string
  end
end
