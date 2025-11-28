# frozen_string_literal: true

class AddTimestampDefaultsToRodauthTables < ActiveRecord::Migration[8.0]
  def change
    # Make timestamps nullable for Rodauth tables since Sequel doesn't set them automatically
    # These tables are managed by Rodauth/Sequel, not ActiveRecord

    change_column_null :account_verification_keys, :created_at, true
    change_column_null :account_verification_keys, :updated_at, true

    change_column_null :account_password_reset_keys, :created_at, true
    change_column_null :account_password_reset_keys, :updated_at, true

    change_column_null :account_login_change_keys, :created_at, true
    change_column_null :account_login_change_keys, :updated_at, true

    change_column_null :account_remember_keys, :created_at, true
    change_column_null :account_remember_keys, :updated_at, true
  end
end
