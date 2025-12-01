# frozen_string_literal: true

class AddTimestampDefaultsToRodauthTables < ActiveRecord::Migration[8.0]
  def change
    # Make timestamps nullable for Rodauth tables since Sequel doesn't set them automatically
    # These tables are managed by Rodauth/Sequel, not ActiveRecord

    change_table :account_verification_keys, bulk: true do |t|
      t.change_null :created_at, true
      t.change_null :updated_at, true
    end

    change_table :account_password_reset_keys, bulk: true do |t|
      t.change_null :created_at, true
      t.change_null :updated_at, true
    end

    change_table :account_login_change_keys, bulk: true do |t|
      t.change_null :created_at, true
      t.change_null :updated_at, true
    end

    change_table :account_remember_keys, bulk: true do |t|
      t.change_null :created_at, true
      t.change_null :updated_at, true
    end
  end
end
