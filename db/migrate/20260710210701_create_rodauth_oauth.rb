# frozen_string_literal: true

class CreateRodauthOauth < ActiveRecord::Migration[8.1]
  def change
    create_table :oauth_applications do |t|
      t.references :account, foreign_key: true
      t.string :name, null: false
      t.string :redirect_uri, null: false
      t.string :client_id, null: false, index: { unique: true }
      t.string :client_secret
      t.string :client_secret_hash
      t.string :scopes, null: false
      t.timestamps
    end

    create_table :oauth_grants do |t|
      t.references :account, null: false, foreign_key: true
      t.references :household_membership, null: false, foreign_key: true
      t.references :person, null: false, foreign_key: true
      t.references :oauth_application, null: false, foreign_key: true
      t.string :type
      t.string :code
      t.string :token, index: { unique: true }
      t.string :token_hash, index: { unique: true }
      t.string :refresh_token, index: { unique: true }
      t.string :refresh_token_hash, index: { unique: true }
      t.datetime :expires_in, null: false
      t.string :redirect_uri
      t.datetime :revoked_at
      t.string :scopes, null: false
      t.string :access_type, null: false, default: 'offline'
      t.string :code_challenge
      t.string :code_challenge_method
      t.integer :permissions_version, null: false
      t.datetime :last_used_at
      t.timestamps

      t.index %i[oauth_application_id code], unique: true
    end
  end
end
