# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Admin::AuditLogsQuery do
  fixtures :accounts, :people, :users

  let(:scope) { PaperTrail::Version.all }
  let(:admin) { users(:admin) }

  before do
    PaperTrail.request.whodunnit = admin.id
    PaperTrail.request(enabled: true) do
      users(:jane).update!(active: false)
      people(:john).update!(name: 'John Updated')
    end
  end

  describe '#call' do
    it 'filters by item_type, event, and whodunnit in descending order' do
      result = described_class.new(
        scope: scope,
        filters: { item_type: 'User', event: 'update', whodunnit: admin.id.to_s },
        page: 1,
        per_page: 10
      ).call

      expect(result.total_count).to eq(1)
      expect(result.versions.map(&:item_type)).to eq(['User'])
      expect(result.versions.map(&:event)).to eq(['update'])
      expect(result.versions.map(&:whodunnit)).to eq([admin.id.to_s])
    end

    it 'applies page and per-page limits' do
      result = described_class.new(scope: scope, filters: {}, page: 2, per_page: 1).call

      expect(result.total_count).to be >= 2
      expect(result.versions.size).to eq(1)
    end

    it 'includes default filter options even when no matching versions exist' do
      result = described_class.new(scope: PaperTrail::Version.none, filters: {}, page: 1, per_page: 10).call

      expect(result.item_types).to include('AuthenticationToken', 'Medication')
      expect(result.events).to include('auth_token/api_session/created', 'update')
    end

    it 'returns filter options from all PaperTrail versions' do
      insert_unlisted_audit_record

      result = described_class.new(scope: scope, filters: {}, page: 1, per_page: 10).call

      expect(result.item_types).to include('Person', 'UnlistedAuditRecord', 'User')
      expect(result.events).to include('custom/audit_event', 'update')
    end
  end

  def insert_unlisted_audit_record
    PaperTrail::Version.connection.execute(
      PaperTrail::Version.sanitize_sql_array(
        [
          'INSERT INTO versions (item_type, item_id, event, object, created_at) VALUES (?, ?, ?, ?, ?)',
          'UnlistedAuditRecord',
          123,
          'custom/audit_event',
          { changed: true }.to_json,
          Time.current.to_fs(:db)
        ]
      )
    )
  end
end
