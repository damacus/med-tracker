# frozen_string_literal: true

class CreateApiAppTokens < ActiveRecord::Migration[8.1]
  def change
    create_table :api_app_tokens do |t|
      t.references :account, null: false, foreign_key: true
      t.references :household_membership, null: false, foreign_key: true
      t.string :token_digest, null: false
      t.string :name, null: false
      t.integer :permissions_version, null: false, default: 1
      t.datetime :last_used_at, null: false
      t.datetime :revoked_at

      t.timestamps
    end

    add_index :api_app_tokens, :token_digest, unique: true
    add_index :api_app_tokens, %i[household_membership_id revoked_at],
              name: 'index_api_app_tokens_on_membership_and_revoked_at'
  end
end
