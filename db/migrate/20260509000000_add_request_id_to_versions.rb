# frozen_string_literal: true

class AddRequestIdToVersions < ActiveRecord::Migration[8.1]
  def change
    add_column :versions, :request_id, :string
    add_index :versions, :request_id
  end
end
