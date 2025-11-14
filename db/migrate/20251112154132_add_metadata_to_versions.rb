# frozen_string_literal: true

# Add metadata column to store additional information like IP address
class AddMetadataToVersions < ActiveRecord::Migration[8.1]
  def change
    add_column :versions, :ip, :string
  end
end
