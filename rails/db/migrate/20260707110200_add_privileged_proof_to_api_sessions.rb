# frozen_string_literal: true

class AddPrivilegedProofToApiSessions < ActiveRecord::Migration[8.1]
  def change
    add_column :api_sessions, :mfa_verified_at, :datetime
    add_column :api_sessions, :oidc_mfa_verified, :boolean, null: false, default: false
    add_index :api_sessions, :mfa_verified_at
  end
end
