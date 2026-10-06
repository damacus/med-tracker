# frozen_string_literal: true

class CreatePlatformSupportAccess < ActiveRecord::Migration[8.1]
  def change
    create_table :platform_admins do |t|
      t.references :account, null: false, foreign_key: true, index: { unique: true }
      t.string :status, null: false, default: 'active'

      t.timestamps
    end

    create_table :support_access_sessions do |t|
      t.references :platform_admin, null: false, foreign_key: true
      t.references :household, null: false, foreign_key: true
      t.text :reason, null: false
      t.datetime :mfa_verified_at, null: false
      t.datetime :starts_at, null: false
      t.datetime :expires_at, null: false
      t.datetime :ended_at
      t.string :request_id
      t.string :ip

      t.timestamps
    end

    add_index :support_access_sessions, %i[household_id expires_at]
    add_index :support_access_sessions, %i[platform_admin_id ended_at]
  end
end
