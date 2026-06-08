# frozen_string_literal: true

class AddDefaultsToAccountWebauthnUserIdTimestamps < ActiveRecord::Migration[8.1]
  def change
    change_column_default :account_webauthn_user_ids, :created_at, from: nil, to: -> { 'CURRENT_TIMESTAMP' }
    change_column_default :account_webauthn_user_ids, :updated_at, from: nil, to: -> { 'CURRENT_TIMESTAMP' }
  end
end
