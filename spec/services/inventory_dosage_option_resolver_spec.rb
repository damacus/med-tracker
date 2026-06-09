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

  let(:inventory) do
    dbl = instance_double(Medication, id: 99, blank?: false, present?: true)
    allow(dbl).to receive_message_chain(:dosage_records, :where, :not, :to_a).and_return([dosage_record])
    dbl
  end

  let(:source) do
    instance_double(
      Schedule,
      respond_to?: false,
      source_dosage_option: nil,
      default_dose_amount: 500,
      dose_unit: 'mg'
    )
  end

  # Allow `respond_to?` selectively on `source`
  before do
    allow(source).to receive(:respond_to?).with(:source_dosage_option).and_return(false)
    allow(source).to receive(:respond_to?).with(:schedule_type_tapering?).and_return(false)
    allow(source).to receive(:respond_to?).with(:effective_dose_amount).and_return(false)
    allow(source).to receive(:respond_to?).with(:effective_dose_unit).and_return(false)
  end

  describe '#tracked_inventory?' do
    context 'when there are dosage records with current_supply' do
      it 'returns true' do
        expect(resolver.tracked_inventory?).to be true
      end
    end

    context 'when there are no dosage records with current_supply' do
      before do
        allow(inventory).to receive_message_chain(:dosage_records, :where, :not, :to_a).and_return([])
      end

      it 'returns false' do
        expect(resolver.tracked_inventory?).to be false
      end
    end
  end

  describe '#call' do
    context 'when inventory is blank' do
      let(:inventory) { nil }

      it 'returns nil' do
        expect(described_class.new(inventory: nil, source: source).call).to be_nil
      end
    end

    context 'when source is blank' do
      it 'returns nil' do
        expect(described_class.new(inventory: inventory, source: nil).call).to be_nil
      end
    end

    context 'when there are no tracked dosage records' do
      before do
        allow(inventory).to receive_message_chain(:dosage_records, :where, :not, :to_a).and_return([])
      end

      it 'returns nil' do
        expect(resolver.call).to be_nil
      end
    end

    context 'when source does not respond to source_dosage_option or schedule_type_tapering?' do
      context 'when snapshot matches by amount and unit' do
        before do
          allow(source).to receive(:default_dose_amount).and_return(500)
          allow(source).to receive(:dose_unit).and_return('mg')
        end

        it 'returns the matching dosage record via snapshot' do
          expect(resolver.call).to eq(dosage_record)
        end
      end

      context 'when snapshot does not match' do
        before do
          allow(source).to receive(:default_dose_amount).and_return(250)
          allow(source).to receive(:dose_unit).and_return('mg')
        end

        it 'returns nil' do
          expect(resolver.call).to be_nil
        end
      end

      context 'when there are multiple matching records' do
        let(:other_record) do
          instance_double(
            MedicationDosageOption,
            id: 11,
            amount: 500,
            unit: 'mg',
            current_supply: 5,
            inventory_match_signature: { amount: '500', unit: 'mg' },
            medication_id: 99
          )
        end

        before do
          allow(inventory).to receive_message_chain(:dosage_records, :where, :not, :to_a)
            .and_return([dosage_record, other_record])
          allow(source).to receive(:default_dose_amount).and_return(500)
          allow(source).to receive(:dose_unit).and_return('mg')
        end

        it 'returns nil when multiple records match the snapshot' do
          expect(resolver.call).to be_nil
        end
      end
    end

    context 'when source responds to source_dosage_option' do
      before do
        allow(source).to receive(:respond_to?).with(:source_dosage_option).and_return(true)
      end

      context 'when source_dosage_option is nil' do
        before { allow(source).to receive(:source_dosage_option).and_return(nil) }

        it 'falls back to snapshot matching' do
          allow(source).to receive(:default_dose_amount).and_return(500)
          allow(source).to receive(:dose_unit).and_return('mg')
          expect(resolver.call).to eq(dosage_record)
        end
      end

      context 'when source_dosage_option is present and same medication_id' do
        let(:source_option) do
          instance_double(
            MedicationDosageOption,
            id: 10,
            medication_id: 99,
            inventory_match_signature: { amount: '500', unit: 'mg' }
          )
        end

        before do
          allow(source).to receive(:source_dosage_option).and_return(source_option)
        end

        it 'resolves by matching id' do
          expect(resolver.call).to eq(dosage_record)
        end
      end

      context 'when source_dosage_option medication_id differs (cross-medication)' do
        let(:source_option) do
          instance_double(
            MedicationDosageOption,
            id: 20,
            medication_id: 1234,
            inventory_match_signature: { amount: '500', unit: 'mg' }
          )
        end

        before do
          allow(source).to receive(:source_dosage_option).and_return(source_option)
          allow(dosage_record).to receive(:inventory_match_signature)
            .and_return({ amount: '500', unit: 'mg' })
        end

        it 'resolves by matching signature' do
          expect(resolver.call).to eq(dosage_record)
        end
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

      context 'when snapshot matches' do
        it 'returns the record via snapshot path first' do
          expect(resolver.call).to eq(dosage_record)
        end
      end

      context 'when snapshot does not match but source_dosage_option does' do
        let(:source_option) do
          instance_double(
            MedicationDosageOption,
            id: 10,
            medication_id: 99,
            inventory_match_signature: { amount: '500', unit: 'mg' }
          )
        end

        before do
          allow(source).to receive(:effective_dose_amount).with(effective_date).and_return(999)
          allow(source).to receive(:effective_dose_unit).with(effective_date).and_return('mg')
          allow(source).to receive(:respond_to?).with(:source_dosage_option).and_return(true)
          allow(source).to receive(:source_dosage_option).and_return(source_option)
        end

        it 'falls back to source_dosage_option' do
          expect(resolver.call).to eq(dosage_record)
        end
      end

      context 'when effective snapshot amounts are blank' do
        before do
          allow(source).to receive(:effective_dose_amount).with(effective_date).and_return(nil)
          allow(source).to receive(:effective_dose_unit).with(effective_date).and_return('mg')
          allow(source).to receive(:respond_to?).with(:source_dosage_option).and_return(false)
          allow(source).to receive(:default_dose_amount).and_return(nil)
          allow(source).to receive(:dose_unit).and_return('mg')
        end

        it 'returns nil' do
          expect(resolver.call).to be_nil
        end
      end
    end
  end
end
