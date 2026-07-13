class AddHouseholdOwnershipToCarerRelationships < ActiveRecord::Migration[8.1]
  HOUSEHOLD_FOREIGN_KEY = 'fk_carer_relationships_household'
  CARER_FOREIGN_KEY = 'fk_carer_relationships_carer_household'
  PATIENT_FOREIGN_KEY = 'fk_carer_relationships_patient_household'

  def up
    add_reference :carer_relationships, :household, index: false
    verify_legacy_relationships!
    backfill_households
    change_column_null :carer_relationships, :household_id, false
    add_indexes
    add_foreign_keys
    enable_household_rls
  end

  def down
    disable_household_rls
    remove_foreign_keys
    remove_indexes
    remove_column :carer_relationships, :household_id
  end

  private

  def verify_legacy_relationships!
    mismatches = legacy_relationship_mismatches
    return if mismatches.empty?

    ids = mismatches.pluck('id').join(', ')
    raise ActiveRecord::MigrationError,
          "carer relationships have missing or cross-household endpoints: #{ids}"
  end

  def legacy_relationship_mismatches
    select_all(<<~SQL.squish).to_a
      SELECT carer_relationships.id,
             patients.household_id AS patient_household_id,
             carers.household_id AS carer_household_id
      FROM carer_relationships
      LEFT JOIN people patients ON patients.id = carer_relationships.patient_id
      LEFT JOIN people carers ON carers.id = carer_relationships.carer_id
      WHERE patients.household_id IS NULL
         OR carers.household_id IS NULL
         OR patients.household_id <> carers.household_id
      ORDER BY carer_relationships.id
    SQL
  end

  def backfill_households
    execute <<~SQL.squish
      UPDATE carer_relationships
      SET household_id = patients.household_id
      FROM people patients
      WHERE patients.id = carer_relationships.patient_id
    SQL
  end

  def add_indexes
    add_index :carer_relationships, :household_id
    add_index :carer_relationships, %i[id household_id], unique: true
    add_index :carer_relationships,
              %i[household_id carer_id patient_id],
              unique: true,
              name: 'index_carer_relationships_on_household_carer_patient'
    remove_index :carer_relationships, column: %i[carer_id patient_id]
  end

  def remove_indexes
    add_index :carer_relationships, %i[carer_id patient_id], unique: true
    remove_index :carer_relationships, name: 'index_carer_relationships_on_household_carer_patient'
    remove_index :carer_relationships, column: %i[id household_id]
    remove_index :carer_relationships, :household_id
  end

  def add_foreign_keys
    add_foreign_key :carer_relationships,
                    :households,
                    column: :household_id,
                    validate: false,
                    name: HOUSEHOLD_FOREIGN_KEY
    add_endpoint_foreign_key(:carer_id, CARER_FOREIGN_KEY)
    add_endpoint_foreign_key(:patient_id, PATIENT_FOREIGN_KEY)
    validate_foreign_key :carer_relationships, name: HOUSEHOLD_FOREIGN_KEY
    validate_foreign_key :carer_relationships, name: CARER_FOREIGN_KEY
    validate_foreign_key :carer_relationships, name: PATIENT_FOREIGN_KEY
  end

  def add_endpoint_foreign_key(column, name)
    add_foreign_key :carer_relationships,
                    :people,
                    column: [column, :household_id],
                    primary_key: %i[id household_id],
                    validate: false,
                    name: name
  end

  def remove_foreign_keys
    remove_foreign_key :carer_relationships, name: PATIENT_FOREIGN_KEY
    remove_foreign_key :carer_relationships, name: CARER_FOREIGN_KEY
    remove_foreign_key :carer_relationships, name: HOUSEHOLD_FOREIGN_KEY
  end

  def enable_household_rls
    execute 'ALTER TABLE carer_relationships ENABLE ROW LEVEL SECURITY;'
    execute 'ALTER TABLE carer_relationships FORCE ROW LEVEL SECURITY;'
    execute <<~SQL.squish
      CREATE POLICY household_tenant_isolation ON carer_relationships
      USING (household_id = med_tracker.current_household_id())
      WITH CHECK (household_id = med_tracker.current_household_id());
    SQL
  end

  def disable_household_rls
    execute 'DROP POLICY IF EXISTS household_tenant_isolation ON carer_relationships;'
    execute 'ALTER TABLE carer_relationships NO FORCE ROW LEVEL SECURITY;'
    execute 'ALTER TABLE carer_relationships DISABLE ROW LEVEL SECURITY;'
  end
end
