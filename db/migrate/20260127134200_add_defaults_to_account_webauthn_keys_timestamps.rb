# frozen_string_literal: true

class AddDefaultsToAccountWebauthnKeysTimestamps < ActiveRecord::Migration[8.1]
  def up
    change_table :account_webauthn_keys, bulk: true do |t|
      t.change_default :created_at, -> { 'CURRENT_TIMESTAMP' }
      t.change_default :updated_at, -> { 'CURRENT_TIMESTAMP' }
    end

    execute <<~SQL.squish
      UPDATE account_webauthn_keys
      SET created_at = CURRENT_TIMESTAMP
      WHERE created_at IS NULL
    SQL

    execute <<~SQL.squish
      UPDATE account_webauthn_keys
      SET updated_at = CURRENT_TIMESTAMP
      WHERE updated_at IS NULL
    SQL
  end

  def down
    change_table :account_webauthn_keys, bulk: true do |t|
      t.change_default :created_at, nil
      t.change_default :updated_at, nil
    end
  end
end
