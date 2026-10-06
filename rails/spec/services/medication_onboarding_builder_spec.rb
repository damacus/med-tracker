# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationOnboardingBuilder do
  subject(:builder) { described_class.new(prefill: prefill) }

  let(:prefill) { instance_double(MedicationOnboardingPrefill) }

  def stub_prefill_empty
    allow(prefill).to receive(:call).and_return(
      MedicationOnboardingPrefill::Result.new(medication_attributes: {}, dosage_records_attributes: [])
    )
  end

  def stub_prefill_with(medication_attributes:, dosage_records: [])
    allow(prefill).to receive(:call).and_return(
      MedicationOnboardingPrefill::Result.new(
        medication_attributes: medication_attributes,
        dosage_records_attributes: dosage_records
      )
    )
  end

  describe '#build_new' do
    let(:location) { create(:location) }
    let(:medication) { build(:medication, name: nil, location: location) }

    context 'when prefill returns medication attributes' do
      before do
        stub_prefill_with(
          medication_attributes: {
            dose_amount: 1,
            dose_unit: 'tablet',
            current_supply: 30,
            reorder_threshold: 7
          },
          dosage_records: [
            { amount: 1, unit: 'tablet', frequency: 'Daily' }
          ]
        )
      end

      it 'assigns prefill medication attributes onto the medication' do
        builder.build_new(medication: medication, params: { name: 'Vitamin C' })
        expect(medication.dose_amount).to eq(1)
        expect(medication.dose_unit).to eq('tablet')
      end

      it 'builds dosage records from the prefill' do
        builder.build_new(medication: medication, params: { name: 'Vitamin C' })
        expect(medication.dosage_records.length).to eq(1)
        expect(medication.dosage_records.first.amount).to eq(1)
        expect(medication.dosage_records.first.unit).to eq('tablet')
      end

      it 'returns the medication' do
        result = builder.build_new(medication: medication, params: { name: 'Vitamin C' })
        expect(result).to be(medication)
      end
    end

    context 'when prefill returns no dosage records' do
      before do
        stub_prefill_with(medication_attributes: {}, dosage_records: [])
      end

      it 'builds a default dosage record from medication attributes' do
        medication.dose_amount = 500
        medication.dose_unit = 'mg'
        builder.build_new(medication: medication, params: {})
        expect(medication.dosage_records.length).to eq(1)
        expect(medication.dosage_records.first.unit).to eq('mg')
      end
    end

    context 'when medication already has dosage records' do
      before { stub_prefill_empty }

      it 'does not build additional dosage records' do
        medication.dosage_records.build(amount: 1, unit: 'tablet', frequency: 'Daily')
        builder.build_new(medication: medication, params: {})
        expect(medication.dosage_records.length).to eq(1)
      end
    end

    context 'with a valid barcode param' do
      before { stub_prefill_empty }

      it 'assigns barcode when it looks like a valid GTIN' do
        builder.build_new(medication: medication, params: { barcode: '5021265221301' })
        expect(medication.barcode).to eq('5021265221301')
      end

      it 'does not assign barcode when value does not look like a GTIN' do
        builder.build_new(medication: medication, params: { barcode: 'NOT-A-BARCODE' })
        expect(medication.barcode).to be_nil
      end
    end

    context 'with dmd_code in params' do
      before { stub_prefill_empty }

      it 'assigns dmd_code and dmd_system' do
        builder.build_new(
          medication: medication,
          params: { dmd_code: '12345', dmd_system: 'snomed', dmd_concept_class: 'VMP' }
        )
        expect(medication.dmd_code).to eq('12345')
        expect(medication.dmd_system).to eq('snomed')
      end
    end

    context 'without dmd_code in params' do
      before { stub_prefill_empty }

      it 'does not assign dmd attributes' do
        builder.build_new(medication: medication, params: {})
        expect(medication.dmd_code).to be_blank
      end
    end

    it 'applies the params name via assign_attributes (finder pass)' do
      medication.name = 'Existing Name'
      stub_prefill_with(medication_attributes: {}, dosage_records: [])
      builder.build_new(medication: medication, params: { name: 'Params Name' })
      expect(medication.name).to eq('Params Name')
    end
  end

  describe '#merge_create_attributes' do
    context 'when dosage_records_attributes is blank and prefill has dosages' do
      before do
        stub_prefill_with(
          medication_attributes: { category: 'Analgesic' },
          dosage_records: [
            { amount: 500, unit: 'mg', frequency: 'As needed', current_supply: 16, reorder_threshold: 4 }
          ]
        )
      end

      it 'fills in dosage_records_attributes from prefill' do
        attrs = { name: 'Paracetamol' }
        builder.merge_create_attributes(attrs)
        expect(attrs[:dosage_records_attributes]).to be_present
        expect(attrs[:dosage_records_attributes]['0']).to include(amount: 500, unit: 'mg')
      end

      it 'fills in medication_attributes from prefill when blank' do
        attrs = { name: 'Paracetamol' }
        builder.merge_create_attributes(attrs)
        expect(attrs[:category]).to eq('Analgesic')
      end

      it 'does not overwrite an already-set medication attribute' do
        attrs = { name: 'Paracetamol', category: 'Vitamin' }
        builder.merge_create_attributes(attrs)
        expect(attrs[:category]).to eq('Vitamin')
      end
    end

    context 'when dosage_records_attributes is already populated' do
      before do
        stub_prefill_with(
          medication_attributes: {},
          dosage_records: [{ amount: 500, unit: 'mg' }]
        )
      end

      it 'does not overwrite existing dosage_records_attributes' do
        attrs = {
          name: 'Ibuprofen',
          dosage_records_attributes: { '0' => { amount: 200, unit: 'mg' } }
        }
        builder.merge_create_attributes(attrs)
        expect(attrs[:dosage_records_attributes]['0']).to include(amount: 200)
      end
    end

    context 'with explicit inventory override (current_supply present)' do
      before do
        stub_prefill_with(
          medication_attributes: {},
          dosage_records: [
            { amount: 1, unit: 'tablet', current_supply: 30, reorder_threshold: 7 }
          ]
        )
      end

      it 'omits current_supply and reorder_threshold from the merged dosage defaults' do
        attrs = { name: 'Vitamin C', current_supply: '10' }
        builder.merge_create_attributes(attrs)
        merged_dosage = attrs[:dosage_records_attributes]['0']
        expect(merged_dosage).not_to have_key(:current_supply)
        expect(merged_dosage).not_to have_key(:reorder_threshold)
      end
    end

    context 'without an explicit inventory override' do
      before do
        stub_prefill_with(
          medication_attributes: {},
          dosage_records: [
            { amount: 1, unit: 'tablet', current_supply: 30, reorder_threshold: 7 }
          ]
        )
      end

      it 'keeps current_supply and reorder_threshold in merged dosage defaults' do
        attrs = { name: 'Vitamin C' }
        builder.merge_create_attributes(attrs)
        merged_dosage = attrs[:dosage_records_attributes]['0']
        expect(merged_dosage).to include(current_supply: 30, reorder_threshold: 7)
      end
    end

    it 'returns the attrs hash' do
      stub_prefill_empty
      attrs = { name: 'Some Med' }
      result = builder.merge_create_attributes(attrs)
      expect(result).to be(attrs)
    end
  end
end
