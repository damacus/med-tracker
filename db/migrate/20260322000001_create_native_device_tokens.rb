# frozen_string_literal: true

class CreateNativeDeviceTokens < ActiveRecord::Migration[8.0]
  def change
    create_table :native_device_tokens do |t|
      t.references :account, null: false, foreign_key: true
      t.string :device_token, null: false
      t.string :platform, null: false
      t.string :user_agent

      t.timestamps
    end

    add_index :native_device_tokens, :device_token, unique: true
    add_index :native_device_tokens, %i[account_id platform]
  end
end
