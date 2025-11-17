# frozen_string_literal: true

class AddAccountIdToPeople < ActiveRecord::Migration[8.0]
  def change
    # account_id is optional - minors and dependent adults may not have accounts
    add_reference :people, :account, null: true, foreign_key: true, index: true
  end
end
