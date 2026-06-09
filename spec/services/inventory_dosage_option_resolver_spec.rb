# frozen_string_literal: true

require 'rails_helper'

RSpec.describe InventoryDosageOptionResolver do
  subject(:resolver) { described_class.new(inventory: inventory, source: source, effective_date: effective_date) }

  let(:effective_date) { Date.new(2026, 6, 9) }
  let(:dosage_record) do
    instance_double(
      MedicationDosageOption,
      id: 10,
      amount: 500,
      unit: 'mg',
      current_supply: 20,
      inventory_match_signature: { amount: '500', unit: 'mg' },
      medication_id: 99
    )
  end
  let(:inventory) { build_inventory([dosage_record]) }
  let(:source) do
    instance_double(
      Schedule,
      respond_to?: false,
      source_dosage_option: nil,
      default_dose_amount: 500,
      dose_unit: 'mg'
    )
  end

  # Build an inventory double yielding `records` from the dosage_records AR chain.
  # rubocop:disable RSpec/VerifiedDoubles
  def build_inventory(records)
    scope = double('DosageRecordsScope', where: nil, not: nil, to_a: records) # rubocop:disable RSpec/VerifiedDoubles
    allow(scope).to receive_messages(where: scope, not: scope)
    dbl = instance_double(Medication, id: 99, blank?: false, present?: true)
    allow(dbl).to receive(:dosage_records).and_return(scope)
    dbl
  end
  # rubocop:enable RSpec/VerifiedDoubles

  # Allow `respond_to?` selectively on `source`
  before do
    allow(source).to receive(:respond_to?).with(:source_dosage_option).and_return(false)
    allow(source).to receive(:respond_to?).with(:schedule_type_tapering?).and_return(false)
    allow(source).to receive(:respond_to?).with(:effective_dose_amount).and_return(false)
    allow(source).to receive(:respond_to?).with(:effective_dose_unit).and_return(false)
  end

  describe '#tracked_inventory?' do
    it 'returns true when dosage records exist with current_supply' do
      expect(resolver.tracked_inventory?).to be true
    end

    it 'returns false when no dosage records exist' do
      resolver2 = described_class.new(inventory: build_inventory([]), source: source,
                                      effective_date: effective_date)
      expect(resolver2.tracked_inventory?).to be false
    end
  end

  describe '#call' do
    it 'returns nil when inventory is blank' do
      expect(described_class.new(inventory: nil, source: source).call).to be_nil
    end

    it 'returns nil when source is blank' do
      expect(described_class.new(inventory: inventory, source: nil).call).to be_nil
    end

    it 'returns nil when there are no tracked dosage records' do
      resolver2 = described_class.new(inventory: build_inventory([]), source: source,
                                      effective_date: effective_date)
      expect(resolver2.call).to be_nil
    end

    context 'when source does not respond to source_dosage_option or schedule_type_tapering?' do
      it 'returns the matching dosage record when snapshot matches amount and unit' do
        allow(source).to receive_messages(default_dose_amount: 500, dose_unit: 'mg')
        expect(resolver.call).to eq(dosage_record)
      end

      it 'returns nil when snapshot does not match' do
        allow(source).to receive_messages(default_dose_amount: 250, dose_unit: 'mg')
        expect(resolver.call).to be_nil
      end

      it 'returns nil when multiple records match the snapshot' do
        other_record = instance_double(
          MedicationDosageOption,
          id: 11, amount: 500, unit: 'mg', current_supply: 5,
          inventory_match_signature: { amount: '500', unit: 'mg' },
          medication_id: 99
        )
        resolver2 = described_class.new(
          inventory: build_inventory([dosage_record, other_record]),
          source: source,
          effective_date: effective_date
        )
        allow(source).to receive_messages(default_dose_amount: 500, dose_unit: 'mg')
        expect(resolver2.call).to be_nil
      end
    end

    context 'when source responds to source_dosage_option' do
      before { allow(source).to receive(:respond_to?).with(:source_dosage_option).and_return(true) }

      it 'falls back to snapshot when source_dosage_option is nil' do
        allow(source).to receive(:source_dosage_option).and_return(nil)
        allow(source).to receive_messages(default_dose_amount: 500, dose_unit: 'mg')
        expect(resolver.call).to eq(dosage_record)
      end

      it 'resolves by id when source_dosage_option has the same medication_id' do
        source_option = instance_double(
          MedicationDosageOption,
          id: 10,
          medication_id: 99,
          inventory_match_signature: { amount: '500', unit: 'mg' }
        )
        allow(source).to receive(:source_dosage_option).and_return(source_option)
        expect(resolver.call).to eq(dosage_record)
      end

      it 'resolves by signature when source_dosage_option medication_id differs' do
        source_option = instance_double(
          MedicationDosageOption,
          id: 20,
          medication_id: 1234,
          inventory_match_signature: { amount: '500', unit: 'mg' }
        )
        allow(source).to receive(:source_dosage_option).and_return(source_option)
        allow(dosage_record).to receive(:inventory_match_signature)
          .and_return({ amount: '500', unit: 'mg' })
        expect(resolver.call).to eq(dosage_record)
      end
    end

    context 'when source is a tapering schedule (prefer_effective_snapshot?)' do
      before do
        allow(source).to receive(:respond_to?).with(:schedule_type_tapering?).and_return(true)
        allow(source).to receive(:schedule_type_tapering?).and_return(true)
        allow(source).to receive(:respond_to?).with(:effective_dose_amount).and_return(true)
        allow(source).to receive(:respond_to?).with(:effective_dose_unit).and_return(true)
        allow(source).to receive(:effective_dose_amount).with(effective_date).and_return(500)
        allow(source).to receive(:effective_dose_unit).with(effective_date).and_return('mg')
      end

      it 'returns the record via snapshot when snapshot matches' do
        expect(resolver.call).to eq(dosage_record)
      end

      it 'falls back to source_dosage_option when snapshot misses' do
        source_option = instance_double(
          MedicationDosageOption,
          id: 10,
          medication_id: 99,
          inventory_match_signature: { amount: '500', unit: 'mg' }
        )
        allow(source).to receive(:effective_dose_amount).with(effective_date).and_return(999)
        allow(source).to receive(:effective_dose_unit).with(effective_date).and_return('mg')
        allow(source).to receive(:respond_to?).with(:source_dosage_option).and_return(true)
        allow(source).to receive(:source_dosage_option).and_return(source_option)
        expect(resolver.call).to eq(dosage_record)
      end

      it 'returns nil when effective snapshot amount is blank' do
        allow(source).to receive(:effective_dose_amount).with(effective_date).and_return(nil)
        allow(source).to receive(:effective_dose_unit).with(effective_date).and_return('mg')
        allow(source).to receive(:respond_to?).with(:source_dosage_option).and_return(false)
        allow(source).to receive_messages(default_dose_amount: nil, dose_unit: 'mg')
        expect(resolver.call).to be_nil
      end
    end
  end
end
