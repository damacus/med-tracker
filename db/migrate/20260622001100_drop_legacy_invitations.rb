# frozen_string_literal: true

class DropLegacyInvitations < ActiveRecord::Migration[8.1]
  def change
    drop_table :invitation_dependents, if_exists: true do |t|
      t.references :invitation, null: false
      t.references :dependent, null: false
      t.timestamps
    end

    drop_table :invitations, if_exists: true do |t|
      t.string :email
      t.integer :role
      t.string :token_digest
      t.datetime :expires_at
      t.datetime :accepted_at
      t.timestamps
    end
  end
end
