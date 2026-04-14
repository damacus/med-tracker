# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Admin::AuditLogsQuery do
  fixtures :accounts, :people, :users

  let(:scope) { PaperTrail::Version.all }
  let(:admin) { users(:admin) }

  before do
    PaperTrail.request.whodunnit = admin.id
    PaperTrail.request(enabled: true) do
      users(:jane).update!(role: :nurse)
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
  end
end
