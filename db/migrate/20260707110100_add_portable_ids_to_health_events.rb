# frozen_string_literal: true

class AddPortableIdsToHealthEvents < ActiveRecord::Migration[8.1]
  def up
    add_column :health_events, :portable_id, :string
    backfill_portable_ids
    change_column_null :health_events, :portable_id, false
    change_column_default :health_events, :portable_id, from: nil, to: -> { 'gen_random_uuid()::text' }
    add_index :health_events, %i[household_id portable_id], unique: true
  end

  def down
    remove_index :health_events, column: %i[household_id portable_id]
    change_column_default :health_events, :portable_id, from: -> { 'gen_random_uuid()::text' }, to: nil
    remove_column :health_events, :portable_id
  end

  private

  def backfill_portable_ids
    rows = select_values('SELECT id FROM health_events WHERE portable_id IS NULL')

    rows.each do |id|
      execute <<~SQL.squish
        UPDATE health_events
        SET portable_id = #{quote(SecureRandom.uuid)}
        WHERE id = #{quote(id)}
      SQL
    end
  end
end
