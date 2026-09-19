class AddClassificationToPersonAccessGrants < ActiveRecord::Migration[8.1]
  FOREIGN_KEY_NAME = 'fk_person_access_grants_classified_by_membership_household'

  def up
    add_column :person_access_grants, :disposition, :string
    add_column :person_access_grants, :classified_at, :datetime
    add_column :person_access_grants, :classification_reason, :string
    add_reference :person_access_grants, :classified_by_membership,
                  foreign_key: { to_table: :household_memberships }
    add_foreign_key :person_access_grants,
                    :household_memberships,
                    column: %i[classified_by_membership_id household_id],
                    primary_key: %i[id household_id],
                    validate: false,
                    name: FOREIGN_KEY_NAME
    validate_foreign_key :person_access_grants, name: FOREIGN_KEY_NAME
  end

  def down
    remove_foreign_key :person_access_grants, name: FOREIGN_KEY_NAME
    remove_reference :person_access_grants, :classified_by_membership, foreign_key: true
    remove_column :person_access_grants, :classification_reason
    remove_column :person_access_grants, :classified_at
    remove_column :person_access_grants, :disposition
  end
end
