# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmd::ArchiveMigration do
  let(:store) { instance_double(NhsDmd::ArchiveStore) }
  let(:owner_role) { -> { true } }
  let(:scope) { NhsDmdImport.where(id: imports) }
  let(:imports) do
    [
      NhsDmdImport.create!(uploaded_filename: 'first.zip', archive_path: '/legacy/first.zip'),
      NhsDmdImport.create!(uploaded_filename: 'complete.zip', archive_path: '/legacy/complete.zip', status: :completed)
    ]
  end

  before { allow(store).to receive(:convert_legacy) }

  it 'reports every non-terminal legacy reference without mutating by default' do
    result = described_class.new(scope:, store:, owner_role:, path_exists: ->(_) { true },
                                 service_name: :persistent).call

    expect(result.to_h).to include(processed_count: 1, converted_count: 0, failed_count: 0, applied: false)
    expect(store).not_to have_received(:convert_legacy)
  end

  it 'converts each live legacy reference in bounded batches when explicitly applied' do
    allow(scope).to receive(:find_in_batches).and_call_original

    result = described_class.new(scope:, store:, owner_role:, path_exists: ->(_) { true },
                                 service_name: :s3, apply: true).call

    expect(result.to_h).to include(processed_count: 1, converted_count: 1, failed_count: 0, applied: true)
    expect(scope).to have_received(:find_in_batches).with(batch_size: 100)
    expect(store).to have_received(:convert_legacy).once
  end

  it 'reports missing legacy files only as aggregate failures' do
    result = described_class.new(scope:, store:, owner_role:, path_exists: ->(_) { false },
                                 service_name: :persistent).call

    expect(result.to_h).to include(processed_count: 1, converted_count: 0, failed_count: 1)
    expect(result.to_h.to_json).not_to include('/legacy/', 'first.zip')
  end

  it 'requires the owner-capable database role' do
    migration = described_class.new(scope:, store:, owner_role: -> { false }, service_name: :persistent)

    expect { migration.call }.to raise_error(SecurityError, 'archive migration requires the owner database role')
  end
end
