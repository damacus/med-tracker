# frozen_string_literal: true

class AddCreatedByMembershipToMedications < ActiveRecord::Migration[8.1]
  def change
    add_column :medications, :created_by_membership_id, :bigint unless column_exists?(
      :medications,
      :created_by_membership_id
    )
    add_index :medications, :created_by_membership_id unless index_exists?(:medications, :created_by_membership_id)

    add_foreign_key :medications,
                    :household_memberships,
                    column: :created_by_membership_id unless foreign_key_exists?(
                      :medications,
                      :household_memberships,
                      column: :created_by_membership_id
                    )
    add_foreign_key :medications,
                    :household_memberships,
                    column: %i[created_by_membership_id household_id],
                    primary_key: %i[id household_id],
                    validate: false,
                    name: 'fk_medications_created_by_membership_id_household' unless foreign_key_exists?(
                      :medications,
                      :household_memberships,
                      name: 'fk_medications_created_by_membership_id_household'
                    )
  end
end
