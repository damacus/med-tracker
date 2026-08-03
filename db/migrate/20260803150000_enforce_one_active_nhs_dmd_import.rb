class EnforceOneActiveNhsDmdImport < ActiveRecord::Migration[8.1]
  INDEX_NAME = 'index_nhs_dmd_imports_one_active'

  def up
    execute <<~SQL
      CREATE UNIQUE INDEX #{INDEX_NAME}
      ON nhs_dmd_imports ((1))
      WHERE status IN (0, 1, 2, 3)
    SQL
  end

  def down
    remove_index :nhs_dmd_imports, name: INDEX_NAME
  end
end
