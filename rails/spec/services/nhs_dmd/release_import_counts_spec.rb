# frozen_string_literal: true

require 'rails_helper'

# Spec-only host that includes the mixin under test so its private helpers are
# exercised. Defined at the top level so that `self.class::Result` resolves.
module NhsDmd
  class ReleaseImportCountsSpecHost
    include ReleaseImportCounts

    Result = Struct.new(
      :created_count,
      :updated_count,
      :unchanged_count,
      :skipped_expired_count,
      :skipped_missing_name_count,
      :skipped_invalid_count,
      keyword_init: true
    )

    public :build_counts, :build_result, :breakdown_payload, :skipped_total

    private

    def count_ampp_records(_file) = 10
    def count_gtin_records(_file) = 5
  end
end

RSpec.describe NhsDmd::ReleaseImportCounts do
  subject(:host) { NhsDmd::ReleaseImportCountsSpecHost.new }

  describe 'COUNT_DEFAULTS' do
    it 'includes all expected counter keys initialised to zero' do
      expected_keys = %i[
        processed ampp_processed ampp_named ampp_skipped gtin_processed
        created updated unchanged skipped_expired skipped_missing_name skipped_invalid
      ]

      expect(described_class::COUNT_DEFAULTS.keys).to match_array(expected_keys)
      expect(described_class::COUNT_DEFAULTS.values).to all(eq(0))
    end

    it 'is frozen' do
      expect(described_class::COUNT_DEFAULTS).to be_frozen
    end
  end

  describe '#build_counts' do
    subject(:counts) { host.build_counts('ampp.xml', 'gtin.xml') }

    it 'merges totals from the two file counters' do
      expect(counts[:ampp_total]).to eq(10)
      expect(counts[:gtin_total]).to eq(5)
      expect(counts[:total]).to eq(15)
    end

    it 'sets all default counters to zero' do
      described_class::COUNT_DEFAULTS.each_key do |key|
        expect(counts[key]).to eq(0), "expected counts[#{key}] to be 0"
      end
    end
  end

  describe '#build_result' do
    subject(:result) { host.build_result(counts) }

    let(:counts) do
      { created: 3, updated: 2, unchanged: 1, skipped_expired: 1, skipped_missing_name: 0, skipped_invalid: 1 }
    end

    it 'sets created_count' do
      expect(result.created_count).to eq(3)
    end

    it 'sets updated_count' do
      expect(result.updated_count).to eq(2)
    end

    it 'sets unchanged_count' do
      expect(result.unchanged_count).to eq(1)
    end

    it 'sets skipped_expired_count' do
      expect(result.skipped_expired_count).to eq(1)
    end

    it 'sets skipped_missing_name_count' do
      expect(result.skipped_missing_name_count).to eq(0)
    end

    it 'sets skipped_invalid_count' do
      expect(result.skipped_invalid_count).to eq(1)
    end
  end

  describe '#breakdown_payload' do
    subject(:payload) { host.breakdown_payload(counts) }

    let(:counts) do
      {
        created: 4, updated: 3, unchanged: 2,
        skipped_expired: 1, skipped_missing_name: 2, skipped_invalid: 3
      }
    end

    it 'calculates imported_count as created + updated' do
      expect(payload[:imported_count]).to eq(7)
    end

    it 'calculates skipped_count as the sum of all skip reasons' do
      expect(payload[:skipped_count]).to eq(6)
    end

    it 'includes individual count fields' do
      expect(payload).to include(
        created_count: 4,
        updated_count: 3,
        unchanged_count: 2,
        skipped_expired_count: 1,
        skipped_missing_name_count: 2,
        skipped_invalid_count: 3
      )
    end
  end

  describe '#skipped_total' do
    it 'sums expired, missing-name, and invalid skips' do
      counts = { skipped_expired: 1, skipped_missing_name: 2, skipped_invalid: 3 }

      expect(host.skipped_total(counts)).to eq(6)
    end

    it 'returns zero when all skip counters are zero' do
      counts = { skipped_expired: 0, skipped_missing_name: 0, skipped_invalid: 0 }

      expect(host.skipped_total(counts)).to eq(0)
    end
  end
end
