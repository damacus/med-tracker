# frozen_string_literal: true

class CreateRodauthOtpRecoveryCodes < ActiveRecord::Migration[8.1]
  def change
    create_table :account_otp_keys, id: false do |t|
      t.bigint :id, primary_key: true # rubocop:disable Rails/DangerousColumnNames
      t.foreign_key :accounts, column: :id
      t.string :key, null: false
      t.integer :num_failures, null: false, default: 0
      t.datetime :last_use, null: false, default: -> { 'CURRENT_TIMESTAMP' }
    end

    # rubocop:disable Rails/CreateTableWithTimestamps, Rails/DangerousColumnNames
    create_table :account_recovery_codes, primary_key: %i[id code] do |t|
      t.bigint :id
      t.foreign_key :accounts, column: :id
      t.string :code
    end
    # rubocop:enable Rails/CreateTableWithTimestamps, Rails/DangerousColumnNames
  end
end
