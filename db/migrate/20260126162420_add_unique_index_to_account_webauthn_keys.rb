# frozen_string_literal: true

class AddUniqueIndexToAccountWebauthnKeys < ActiveRecord::Migration[8.1]
  def change
    # Add unique index on webauthn_id scoped to account_id
    remove_index :account_webauthn_keys, :webauthn_id if index_exists?(:account_webauthn_keys, :webauthn_id)
    add_index :account_webauthn_keys, %i[webauthn_id account_id], unique: true, name: 'index_account_webauthn_keys_on_webauthn_id_and_account_id'
  end
end
