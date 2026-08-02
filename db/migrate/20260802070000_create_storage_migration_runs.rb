class CreateStorageMigrationRuns < ActiveRecord::Migration[8.1]
  def change
    create_table :storage_migration_runs do |t|
      t.uuid :run_id, null: false, default: -> { 'gen_random_uuid()' }
      t.string :source_service_name, null: false
      t.string :destination_service_name, null: false
      t.string :phase, null: false, default: 'backfill'
      t.bigint :processed_count, null: false, default: 0
      t.bigint :verified_count, null: false, default: 0
      t.bigint :failed_count, null: false, default: 0
      t.bigint :stable_blob_count
      t.datetime :reconciled_at
      t.datetime :mirror_queue_drained_at
      t.datetime :rollback_deadline
      t.datetime :acceptance_verified_at
      t.datetime :recovery_verified_at
      t.datetime :final_reconciled_at
      t.datetime :finalized_at
      t.timestamps
    end

    add_index :storage_migration_runs, :run_id, unique: true
  end
end
