# frozen_string_literal: true

class EnforceOneActiveNhsDmdImport < ActiveRecord::Migration[8.1]
  INDEX_NAME = 'index_nhs_dmd_imports_one_active'
  INTERRUPTION_MESSAGE = 'Import interrupted because the worker stopped reporting progress.'

  def up
    reconcile_interrupted_imports

    execute <<~SQL
      CREATE UNIQUE INDEX #{INDEX_NAME}
      ON nhs_dmd_imports ((1))
      WHERE status IN (0, 1, 2, 3)
    SQL
  end

  def down
    remove_index :nhs_dmd_imports, name: INDEX_NAME
  end

  private

  def reconcile_interrupted_imports
    execute <<~SQL
      WITH active_imports AS (
        SELECT id, updated_at,
               ROW_NUMBER() OVER (ORDER BY updated_at DESC, id DESC) AS recency
        FROM nhs_dmd_imports
        WHERE status IN (0, 1, 2, 3)
      ), interrupted_imports AS (
        SELECT id
        FROM active_imports
        WHERE updated_at <= CURRENT_TIMESTAMP - INTERVAL '30 minutes'
           OR recency > 1
      )
      UPDATE nhs_dmd_imports
      SET status = 5,
          completed_at = CURRENT_TIMESTAMP,
          updated_at = CURRENT_TIMESTAMP,
          error_message = #{connection.quote(INTERRUPTION_MESSAGE)},
          log = CONCAT_WS(E'\n', NULLIF(log, ''), #{connection.quote(INTERRUPTION_MESSAGE)})
      WHERE id IN (SELECT id FROM interrupted_imports)
    SQL
  end
end
