# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationTradeFamilyResolver do
  subject(:resolver) { described_class.new(scope: scope) }

  let(:medications) do
    {
      exact: create(:medication, barcode: '5016298210989'),
      related: create(:medication, barcode: '5016298210996'),
      zero_prefixed: create(:medication, barcode: '05016298211016'),
      unprefixed: create(:medication, barcode: '5016298211023'),
      unrelated: create(:medication, barcode: '5016298211009')
    }
  end
  let(:scope) { Medication.where(id: medications.values.pluck(:id)) }

  def exact_medication = medications.fetch(:exact)
  def related_medication = medications.fetch(:related)
  def zero_prefixed_medication = medications.fetch(:zero_prefixed)
  def unprefixed_medication = medications.fetch(:unprefixed)
  def unrelated_medication = medications.fetch(:unrelated)

  before do
    matching_family = NhsDmdTradeFamily.create!(code: 'TF001', name: 'Laxido')
    unrelated_family = NhsDmdTradeFamily.create!(code: 'TF002', name: 'Other medicine')
    NhsDmdAmpTradeFamily.create!(amp_code: 'AMP001', trade_family: matching_family)
    NhsDmdAmpTradeFamily.create!(amp_code: 'AMP002', trade_family: matching_family)
    NhsDmdAmpTradeFamily.create!(amp_code: 'AMP003', trade_family: unrelated_family)
    NhsDmdAmpTradeFamily.create!(amp_code: 'AMP004', trade_family: matching_family)
    NhsDmdAmpTradeFamily.create!(amp_code: 'AMP005', trade_family: matching_family)
    NhsDmdBarcode.create!(
      gtin: exact_medication.barcode, code: 'AMPP001', amp_code: 'AMP001', display: exact_medication.name
    )
    NhsDmdBarcode.create!(
      gtin: related_medication.barcode, code: 'AMPP002', amp_code: 'AMP002', display: related_medication.name
    )
    NhsDmdBarcode.create!(
      gtin: unrelated_medication.barcode, code: 'AMPP003', amp_code: 'AMP003', display: unrelated_medication.name
    )
    NhsDmdBarcode.create!(
      gtin: '5016298211016', code: 'AMPP004', amp_code: 'AMP004', display: zero_prefixed_medication.name
    )
    NhsDmdBarcode.create!(
      gtin: '05016298211023', code: 'AMPP005', amp_code: 'AMP005', display: unprefixed_medication.name
    )
  end

  describe '#call' do
    it 'groups household medicines by each requested trade family' do
      related_medications = resolver.call(trade_family_codes: %w[TF001 TF002])

      expect(related_medications.fetch('TF001')).to include(exact_medication, related_medication)
      expect(related_medications.fetch('TF002')).to contain_exactly(unrelated_medication)
    end

    it 'matches household barcodes across 13 and zero-prefixed 14 digit GTIN forms' do
      expect(resolver.call(trade_family_codes: ['TF001']).fetch('TF001')).to include(
        zero_prefixed_medication,
        unprefixed_medication
      )
    end

    it 'loads requested families and their locations in one batched query pair' do
      scope
      queries = capture_catalogue_queries do
        related_medications = resolver.call(trade_family_codes: %w[TF001 TF002])
        related_medications.values.flatten.each { |medication| medication.location.name }
      end

      expect(queries.count { |sql| sql.include?('FROM "medications"') }).to eq(1)
      expect(queries.count { |sql| sql.include?('FROM "locations"') }).to eq(1)
    end

    it 'returns no groups when no trade family codes are supplied' do
      expect(resolver.call(trade_family_codes: [nil, ''])).to eq({})
    end
  end

  def capture_catalogue_queries(&)
    queries = []
    subscriber = lambda do |_name, _start, _finish, _id, payload|
      next if payload[:cached] || payload[:name] == 'SCHEMA'

      queries << payload[:sql]
    end

    ActiveRecord::Base.uncached do
      ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
    end
    queries
  end
end
