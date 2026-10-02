# frozen_string_literal: true

class CreateAccountWebauthnAuthChallenges < ActiveRecord::Migration[8.1]
  def change
    create_table :account_webauthn_auth_challenges do |t|
      t.string :challenge_digest, null: false
      t.datetime :created_at, null: false
    end
    add_index :account_webauthn_auth_challenges, :challenge_digest, unique: true
  end
end
