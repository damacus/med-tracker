# frozen_string_literal: true

class CreateDependentAccessRequests < ActiveRecord::Migration[8.1]
  def change
    create_table :dependent_access_requests do |t|
      t.references :requester, null: false, foreign_key: { to_table: :users }
      t.references :reviewer, foreign_key: { to_table: :users }
      t.references :carer, null: false, foreign_key: { to_table: :people }, index: true
      t.references :patient, null: false, foreign_key: { to_table: :people }, index: true
      t.integer :status, null: false, default: 0
      t.string :relationship_type, null: false, default: 'parent'
      t.datetime :reviewed_at

      t.timestamps
    end

    add_index :dependent_access_requests, :status
    add_index :dependent_access_requests,
              %i[carer_id patient_id],
              unique: true,
              where: 'status = 0',
              name: 'index_dependent_access_requests_on_pending_pair'
  end
end
