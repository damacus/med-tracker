# frozen_string_literal: true

class AddHouseholdKeyToInvitationGrants < ActiveRecord::Migration[8.1]
  def change
    add_reference :household_invitation_grants, :household, foreign_key: true unless column_exists?(:household_invitation_grants, :household_id)

    reversible do |dir|
      dir.up do
        execute <<~SQL
          UPDATE household_invitation_grants
          SET household_id = household_invitations.household_id
          FROM household_invitations
          WHERE household_invitation_grants.household_invitation_id = household_invitations.id
            AND household_invitation_grants.household_id IS NULL;
        SQL
      end
    end

    change_column_null :household_invitation_grants, :household_id, false
    remove_index :household_invitation_grants, column: %i[id household_id], if_exists: true
    add_index :household_invitation_grants, %i[id household_id], unique: true

    add_foreign_key :household_invitation_grants,
                    :household_invitations,
                    column: %i[household_invitation_id household_id],
                    primary_key: %i[id household_id],
                    validate: false,
                    name: 'fk_household_invitation_grants_invitation_household'
    add_foreign_key :household_invitation_grants,
                    :people,
                    column: %i[person_id household_id],
                    primary_key: %i[id household_id],
                    validate: false,
                    name: 'fk_household_invitation_grants_person_household'
  end
end
