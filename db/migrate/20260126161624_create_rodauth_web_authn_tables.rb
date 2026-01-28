# frozen_string_literal: true

class CreateRodauthWebAuthnTables < ActiveRecord::Migration[8.1]
  def change
    create_table :account_webauthn_user_ids do |t|
      t.bigint :account_id, null: false
      t.string :webauthn_id, null: false
      t.timestamps
    end
    add_index :account_webauthn_user_ids, :account_id
    add_index :account_webauthn_user_ids, :webauthn_id, unique: true

    create_table :account_webauthn_keys do |t|
      t.bigint :account_id, null: false
      t.string :webauthn_id, null: false
      t.string :public_key, null: false
      t.integer :sign_count, null: false, default: 0
      t.datetime :last_use
      t.string :nickname
      t.timestamps
    end
    add_index :account_webauthn_keys, :account_id
    add_index :account_webauthn_keys, :webauthn_id
  end
end
