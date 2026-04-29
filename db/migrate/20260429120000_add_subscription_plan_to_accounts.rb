# frozen_string_literal: true

class AddSubscriptionPlanToAccounts < ActiveRecord::Migration[8.1]
  def change
    add_column :accounts, :subscription_plan, :string, null: false, default: 'free'
  end
end
