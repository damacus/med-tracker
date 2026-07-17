# frozen_string_literal: true

class CreateApiHouseholdSelectionGrants < ActiveRecord::Migration[8.1]
  def change
    create_table :api_household_selection_grants do |t|
      t.references :account, null: false, foreign_key: true
      t.string :token_digest, null: false
      t.boolean :oidc_mfa_verified, null: false, default: false
      t.datetime :mfa_verified_at
      t.string :device_name
      t.string :user_agent
      t.datetime :expires_at, null: false
      t.datetime :used_at
      t.timestamps
    end

    add_index :api_household_selection_grants, :token_digest, unique: true
    add_index :api_household_selection_grants, :expires_at
    add_index :api_household_selection_grants, :used_at
  end
end
