# frozen_string_literal: true

require 'rails_helper'
require Rails.root.join('db/migrate/20260919131000_retire_nhs_dmd_archive_path')

RSpec.describe RetireNhsDmdArchivePath do
  self.use_transactional_tests = false

  let(:connection) { ActiveRecord::Base.connection }
  let(:migration) { described_class.new }

  around do |example|
    connection.transaction(requires_new: true) do
      migrate_down
      example.run
      raise ActiveRecord::Rollback
    end
  ensure
    connection.schema_cache.clear!
    NhsDmdImport.reset_column_information
  end

  it 'refuses to remove a nonempty archive path' do
    import_id = connection.select_value(<<~SQL.squish)
      INSERT INTO nhs_dmd_imports (uploaded_filename, archive_path, status, created_at, updated_at)
      VALUES ('legacy.zip', '/legacy/release.zip', 4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
      RETURNING id
    SQL

    expect { migrate_up }
      .to raise_error(ActiveRecord::MigrationError, /nhs_dmd_imports contains 1 nonempty archive_path/)
    expect(connection.column_exists?(:nhs_dmd_imports, :archive_path)).to be(true)
  ensure
    connection.execute("DELETE FROM nhs_dmd_imports WHERE id = #{connection.quote(import_id)}") if import_id
  end

  it 'removes the column when all stored paths are empty' do
    connection.execute(<<~SQL.squish)
      INSERT INTO nhs_dmd_imports (uploaded_filename, archive_path, status, created_at, updated_at)
      VALUES ('blank.zip', '', 4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
    SQL

    migrate_up

    expect(connection.column_exists?(:nhs_dmd_imports, :archive_path)).to be(false)
  end

  it 'restores the original column definition on rollback' do
    migrate_up
    migrate_down

    column = connection.columns(:nhs_dmd_imports).find { it.name == 'archive_path' }
    expect([column.type, column.null, column.default]).to eq([:string, true, nil])
  end

  def migrate_up
    ActiveRecord::Migration.suppress_messages { migration.migrate(:up) }
  end

  def migrate_down
    return if connection.column_exists?(:nhs_dmd_imports, :archive_path)

    ActiveRecord::Migration.suppress_messages { migration.migrate(:down) }
  end
end
