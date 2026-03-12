class AddPreferencesToUsers < ActiveRecord::Migration[8.0]
  def change
    add_column :users, :preferences, :jsonb, null: false, default: {}
    add_index :users, :preferences, using: :gin
  end
end
