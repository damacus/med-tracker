# frozen_string_literal: true

class CreateHostedHouseholdLifecycle < ActiveRecord::Migration[8.1]
  TENANT_TABLES = %w[household_exports household_retention_holds].freeze

  def up
    add_column :households, :lifecycle_state, :string, null: false, default: 'active'
    add_column :households, :offboarded_at, :datetime
    add_index :households, :lifecycle_state

    create_household_exports
    create_household_retention_holds
    create_household_purge_runs
    enable_tenant_isolation
  end

  def down
    drop_table :household_purge_runs
    drop_table :household_retention_holds
    drop_table :household_exports
    remove_column :households, :offboarded_at
    remove_column :households, :lifecycle_state
  end

  private

  def create_household_exports
    create_table :household_exports do |t|
      t.references :household, null: false, foreign_key: { deferrable: :deferred }
      t.references :requested_by_account, null: false, foreign_key: { to_table: :accounts }
      t.string :status, null: false, default: 'requested'
      t.jsonb :manifest, null: false, default: {}
      t.string :artifact_checksum_sha256
      t.bigint :artifact_byte_size
      t.datetime :requested_at, null: false
      t.datetime :generation_started_at
      t.datetime :ready_at
      t.datetime :downloaded_at
      t.datetime :expires_at
      t.datetime :expired_at
      t.datetime :failed_at
      t.string :failure_code
      t.timestamps
    end
    add_index :household_exports, %i[id household_id], unique: true
    add_index :household_exports, %i[household_id status]
    add_index :household_exports, :expires_at
  end

  def create_household_retention_holds
    create_table :household_retention_holds do |t|
      t.references :household, null: false, foreign_key: { deferrable: :deferred }
      t.references :approved_by_account, null: false, foreign_key: { to_table: :accounts }
      t.references :released_by_account, foreign_key: { to_table: :accounts }
      t.string :status, null: false, default: 'active'
      t.text :reason, null: false
      t.date :review_on, null: false
      t.datetime :placed_at, null: false
      t.datetime :released_at
      t.timestamps
    end
    add_index :household_retention_holds, %i[id household_id], unique: true
    add_index :household_retention_holds, :household_id, unique: true, where: "status = 'active'",
                                                             name: 'idx_one_active_retention_hold_per_household'
    add_index :household_retention_holds, %i[status review_on]
  end

  def create_household_purge_runs
    create_table :household_purge_runs do |t|
      t.references :household, null: false, foreign_key: { deferrable: :deferred }
      t.references :requested_by_account, null: false, foreign_key: { to_table: :accounts }
      t.string :status, null: false, default: 'pending'
      t.string :last_completed_table
      t.string :failure_code
      t.integer :attempts, null: false, default: 0
      t.datetime :started_at
      t.datetime :completed_at
      t.datetime :failed_at
      t.timestamps
    end
    add_index :household_purge_runs, %i[household_id status]
  end

  def enable_tenant_isolation
    TENANT_TABLES.each do |table_name|
      execute "ALTER TABLE #{table_name} ENABLE ROW LEVEL SECURITY;"
      execute "ALTER TABLE #{table_name} FORCE ROW LEVEL SECURITY;"
      execute <<~SQL.squish
        CREATE POLICY household_tenant_isolation ON #{table_name}
        USING (household_id = med_tracker.current_household_id())
        WITH CHECK (household_id = med_tracker.current_household_id());
      SQL
    end
  end
end
