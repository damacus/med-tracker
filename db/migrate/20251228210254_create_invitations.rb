# frozen_string_literal: true

class CreateInvitations < ActiveRecord::Migration[8.1]
  def change
    create_table :invitations do |t|
      t.string :email
      t.integer :role
      t.string :token
      t.datetime :expires_at
      t.datetime :accepted_at

      t.timestamps
    end
    add_index :invitations, :token
  end
end
