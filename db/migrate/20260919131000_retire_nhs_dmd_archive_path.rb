# frozen_string_literal: true

class RetireNhsDmdArchivePath < ActiveRecord::Migration[8.1]
  def up
    execute 'LOCK TABLE nhs_dmd_imports IN ACCESS EXCLUSIVE MODE'
    path_count = select_value(<<~SQL.squish).to_i
      SELECT COUNT(*) FROM nhs_dmd_imports WHERE archive_path IS NOT NULL AND archive_path <> ''
    SQL
    if path_count.positive?
      raise ActiveRecord::MigrationError,
            "nhs_dmd_imports contains #{path_count} nonempty archive_path value(s); refusing to remove the column"
    end

    remove_column :nhs_dmd_imports, :archive_path
  end

  def down
    add_column :nhs_dmd_imports, :archive_path, :string
  end
end
