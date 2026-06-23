# frozen_string_literal: true

class BindApiSessionsToHouseholdMemberships < ActiveRecord::Migration[8.1]
  def change
    add_reference :api_sessions, :household_membership, foreign_key: true, index: true
    add_column :api_sessions, :permissions_version, :integer, null: false, default: 1
    add_index :api_sessions, %i[household_membership_id revoked_at], name: 'index_api_sessions_on_membership_and_revoked_at'
  end
end
