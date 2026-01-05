# frozen_string_literal: true

class AddUniqueIndexToInvitationsToken < ActiveRecord::Migration[8.1]
  def change
    remove_index :invitations, :token
    add_index :invitations, :token, unique: true
  end
end
