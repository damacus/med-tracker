class AddAccountIdToPeople < ActiveRecord::Migration[8.0]
  def change
    add_reference :people, :account, null: true, foreign_key: true, index: { unique: true }
  end
end

