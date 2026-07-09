# frozen_string_literal: true

class CreateMedicationReviewPrompts < ActiveRecord::Migration[8.1]
  def up
    create_table :medication_review_prompts do |t|
      t.references :household, null: false, foreign_key: { deferrable: :deferred }
      t.references :person, null: false, foreign_key: { deferrable: :deferred }
      t.references :primary_medication, null: false, foreign_key: { to_table: :medications, deferrable: :deferred }
      t.references :interacting_medication, null: false,
                                            foreign_key: { to_table: :medications, deferrable: :deferred }
      t.references :evidence_record, null: false,
                                     foreign_key: { to_table: :medication_review_evidence_records,
                                                    deferrable: :deferred }
      t.references :reviewed_by_membership, foreign_key: { to_table: :household_memberships,
                                                           deferrable: :deferred }
      t.string :status, null: false, default: 'needs_review'
      t.string :risk_level, null: false
      t.string :match_confidence, null: false
      t.string :primary_medication_name, null: false
      t.string :interacting_medication_name, null: false
      t.string :evidence_source_name, null: false
      t.string :evidence_source_url, null: false
      t.date :evidence_source_checked_on, null: false
      t.text :evidence_text, null: false
      t.string :practitioner_name
      t.string :practitioner_role
      t.date :reviewed_on
      t.text :review_note

      t.timestamps
    end

    add_index :medication_review_prompts, %i[id household_id], unique: true
    add_index :medication_review_prompts,
              %i[household_id person_id primary_medication_id interacting_medication_id evidence_record_id],
              unique: true,
              name: 'idx_medication_review_prompts_unique_pair'
    add_index :medication_review_prompts, %i[household_id status]

    add_household_foreign_key(:person_id, :people, 'fk_review_prompts_person_household')
    add_household_foreign_key(:primary_medication_id, :medications, 'fk_review_prompts_primary_medication_household')
    add_household_foreign_key(:interacting_medication_id, :medications,
                              'fk_review_prompts_interacting_medication_household')
    add_household_foreign_key(:reviewed_by_membership_id, :household_memberships,
                              'fk_review_prompts_reviewer_household')
    enable_household_rls
  end

  def down
    drop_table :medication_review_prompts
  end

  private

  def add_household_foreign_key(column, table, name)
    add_foreign_key :medication_review_prompts,
                    table,
                    column: [column, :household_id],
                    primary_key: %i[id household_id],
                    validate: false,
                    name: name
  end

  def enable_household_rls
    execute 'ALTER TABLE medication_review_prompts ENABLE ROW LEVEL SECURITY;'
    execute 'ALTER TABLE medication_review_prompts FORCE ROW LEVEL SECURITY;'
    execute <<~SQL.squish
      CREATE POLICY household_tenant_isolation ON medication_review_prompts
      USING (household_id = med_tracker.current_household_id())
      WITH CHECK (household_id = med_tracker.current_household_id());
    SQL
  end
end
