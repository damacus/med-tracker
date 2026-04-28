# frozen_string_literal: true

class AddDefaultsToAccountIdentitiesTimestamps < ActiveRecord::Migration[8.1]
  def up
    change_column_default :account_identities, :created_at, -> { 'CURRENT_TIMESTAMP' }
    change_column_default :account_identities, :updated_at, -> { 'CURRENT_TIMESTAMP' }

    execute <<~SQL.squish
      UPDATE account_identities
      SET created_at = CURRENT_TIMESTAMP
      WHERE created_at IS NULL
    SQL

    execute <<~SQL.squish
      UPDATE account_identities
      SET updated_at = CURRENT_TIMESTAMP
      WHERE updated_at IS NULL
    SQL
  end

  def down
    change_column_default :account_identities, :created_at, nil
    change_column_default :account_identities, :updated_at, nil
  end
end
