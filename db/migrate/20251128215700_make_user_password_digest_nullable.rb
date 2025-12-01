# frozen_string_literal: true

class MakeUserPasswordDigestNullable < ActiveRecord::Migration[8.0]
  def change
    # Make password_digest nullable for users created via Rodauth
    # Rodauth stores passwords in the accounts table, not users
    change_column_null :users, :password_digest, true
  end
end
