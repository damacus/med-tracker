class AddCarerRelationshipSourceToPersonAccessGrants < ActiveRecord::Migration[8.1]
  FOREIGN_KEY_NAME = 'fk_person_access_grants_carer_relationship_household'

  def up
    add_reference :person_access_grants, :carer_relationship, index: false
    add_index :person_access_grants,
              %i[carer_relationship_id household_id],
              name: 'idx_person_access_grants_on_delegation_household'
    add_foreign_key :person_access_grants,
                    :carer_relationships,
                    column: %i[carer_relationship_id household_id],
                    primary_key: %i[id household_id],
                    validate: false,
                    name: FOREIGN_KEY_NAME
    validate_foreign_key :person_access_grants, name: FOREIGN_KEY_NAME
  end

  def down
    remove_foreign_key :person_access_grants, name: FOREIGN_KEY_NAME
    remove_index :person_access_grants, name: 'idx_person_access_grants_on_delegation_household'
    remove_column :person_access_grants, :carer_relationship_id
  end
end
