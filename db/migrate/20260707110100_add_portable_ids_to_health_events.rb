# frozen_string_literal: true

class AddPortableIdsToHealthEvents < ActiveRecord::Migration[8.1]
  def up
    add_column :health_events, :portable_id, :string unless column_exists?(:health_events, :portable_id)
    prepare_portable_ids
    change_column_null :health_events, :portable_id, false
    change_column_default :health_events, :portable_id, from: nil, to: -> { 'gen_random_uuid()::text' }
    add_index :health_events, %i[household_id portable_id], unique: true unless
      index_exists?(:health_events, %i[household_id portable_id], unique: true)
  end

  def down
    remove_index :health_events, column: %i[household_id portable_id]
    change_column_default :health_events, :portable_id, from: -> { 'gen_random_uuid()::text' }, to: nil
    remove_column :health_events, :portable_id
  end

  private

  def prepare_portable_ids
    with_forced_rls_relaxed do
      backfill_portable_ids
      verify_no_null_portable_ids!
    end
  end

  def backfill_portable_ids
    execute <<~SQL.squish
      UPDATE health_events
      SET portable_id = gen_random_uuid()::text
      WHERE portable_id IS NULL
    SQL
  end

  def verify_no_null_portable_ids!
    count = select_value('SELECT COUNT(*) FROM health_events WHERE portable_id IS NULL').to_i
    return if count.zero?

    raise ActiveRecord::IrreversibleMigration,
          "health_events has #{count} rows without portable_id"
  end

  def with_forced_rls_relaxed
    forced = forced_row_level_security?
    execute 'ALTER TABLE health_events NO FORCE ROW LEVEL SECURITY' if forced
    yield
  ensure
    execute 'ALTER TABLE health_events FORCE ROW LEVEL SECURITY' if forced
  end

  def forced_row_level_security?
    select_value(<<~SQL.squish)
      SELECT relforcerowsecurity
      FROM pg_class
      WHERE oid = 'health_events'::regclass
    SQL
  end
end
