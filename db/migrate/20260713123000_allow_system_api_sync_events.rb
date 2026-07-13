# frozen_string_literal: true

class AllowSystemApiSyncEvents < ActiveRecord::Migration[8.1]
  def change
    change_column_null :api_change_events, :account_id, true
    change_column_null :api_change_events, :household_membership_id, true
    change_column_null :api_tombstones, :account_id, true
    change_column_null :api_tombstones, :household_membership_id, true
  end
end
