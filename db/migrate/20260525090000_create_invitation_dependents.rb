# frozen_string_literal: true

class CreateInvitationDependents < ActiveRecord::Migration[8.1]
  def change
    create_table :invitation_dependents do |t|
      t.references :invitation, null: false, foreign_key: true
      t.references :dependent, null: false, foreign_key: { to_table: :people }

      t.timestamps
    end

    add_index :invitation_dependents, %i[invitation_id dependent_id], unique: true
  end
end
