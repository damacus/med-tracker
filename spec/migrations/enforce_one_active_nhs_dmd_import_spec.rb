# frozen_string_literal: true

require 'rails_helper'
require Rails.root.join('db/migrate/20260803150000_enforce_one_active_nhs_dmd_import')

RSpec.describe EnforceOneActiveNhsDmdImport do
  let(:connection) { ActiveRecord::Base.connection }
  let(:migration) { described_class.new }

  it 'reconciles existing interrupted and duplicate active imports before adding the index' do
    with_index_rebuilt do |imports|
      expect(imports.transform_values { it.reload.status }).to eq(
        stale: 'failed', older_recent: 'failed', latest_recent: 'importing'
      )
      expect(imports.values_at(:stale, :older_recent).map(&:error_message)).to all(eq(interruption_message))
      expect(imports.values_at(:stale, :older_recent).map(&:log)).to all(include(interruption_message))
      expect(connection.index_exists?(:nhs_dmd_imports, name: described_class::INDEX_NAME)).to be(true)
    end
  end

  def with_index_rebuilt
    connection.transaction(requires_new: true) do
      ActiveRecord::Migration.suppress_messages { migration.down }
      imports = create_duplicate_active_imports
      ActiveRecord::Migration.suppress_messages { migration.up }
      yield imports
      raise ActiveRecord::Rollback
    end
  end

  def create_duplicate_active_imports
    {
      stale: create_import_at(31.minutes.ago, 'stale.zip'),
      older_recent: create_import_at(2.minutes.ago, 'older-recent.zip'),
      latest_recent: create_import_at(1.minute.ago, 'latest-recent.zip')
    }
  end

  def create_import_at(time, filename)
    travel_to(time) { NhsDmdImport.create!(uploaded_filename: filename, status: :importing) }
  end

  def interruption_message
    described_class::INTERRUPTION_MESSAGE
  end
end
