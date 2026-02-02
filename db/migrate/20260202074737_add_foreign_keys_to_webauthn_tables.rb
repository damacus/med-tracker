# frozen_string_literal: true

class AddForeignKeysToWebauthnTables < ActiveRecord::Migration[8.0]
  def change
    add_foreign_key :account_webauthn_keys, :accounts, column: :account_id, on_delete: :cascade
    add_foreign_key :account_webauthn_user_ids, :accounts, column: :account_id, on_delete: :cascade
  end
end
