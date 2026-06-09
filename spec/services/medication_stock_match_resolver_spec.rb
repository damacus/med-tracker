# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationStockMatchResolver do
  subject(:resolver) { described_class.new(scope: scope, barcode_lookup: barcode_lookup) }

  let(:medication) { create(:medication, name: 'Paracetamol 500 mg Tablets', dosage_unit: 'tablet', dosage_amount: nil) }
  let(:scope) { Medication.where(id: medication.id) }
  let(:barcode_lookup) { instance_double(BarcodeCatalog::Lookup, lookup: nil) }

  describe '#call' do
    context 'when a direct barcode match exists in scope' do
      before { medication.update!(barcode: '5000168511017') }

      it 'returns the medication without consulting the catalog' do
        result = resolver.call(barcode: '5000168511017', name: nil)
        expect(result).to eq(medication)
        expect(barcode_lookup).not_to have_received(:lookup)
      end
    end

    context 'when no direct barcode match but catalog returns a match' do
      let(:catalog_attrs) do
        { code: 'DMD123', system: 'snomed', concept_class: 'VMP', display: 'Paracetamol 500 mg Tablets' }
      end

      before do
        allow(barcode_lookup).to receive(:lookup).with('12345678901234').and_return(catalog_attrs)
      end

      it 'builds a candidate from catalog data and delegates to MedicationInventoryMatcher' do
        result = resolver.call(barcode: '12345678901234', name: nil)
        expect(result).to eq(medication)
      end
    end

    context 'when no barcode and name is provided' do
      it 'builds a candidate from attributes and matches by name' do
        result = resolver.call(barcode: nil, name: 'Paracetamol 500 mg Tablets')
        expect(result).to eq(medication)
      end
    end

    context 'when display attribute is used as name fallback' do
      it 'matches using display when name is absent' do
        result = resolver.call(barcode: nil, display: 'Paracetamol 500 mg Tablets')
        expect(result).to eq(medication)
      end
    end

    context 'when all identifying attributes are blank' do
      it 'returns nil' do
        result = resolver.call(barcode: nil, name: nil)
        expect(result).to be_nil
      end
    end

    context 'when barcode normalisation strips non-digits' do
      before { medication.update!(barcode: '5000168511017') }

      it 'normalises barcode with dashes before matching' do
        result = resolver.call(barcode: '500-016-8511017', name: nil)
        expect(result).to eq(medication)
      end
    end

    context 'when barcode is blank' do
      it 'does not consult the catalog for blank barcodes' do
        resolver.call(barcode: '', name: 'Paracetamol 500 mg Tablets')
        expect(barcode_lookup).not_to have_received(:lookup)
      end
    end

    context 'when catalog returns nil' do
      before do
        allow(barcode_lookup).to receive(:lookup).and_return(nil)
      end

      it 'still matches by name when provided' do
        result = resolver.call(barcode: '99999999999999', name: 'Paracetamol 500 mg Tablets')
        expect(result).to eq(medication)
      end
    end

    context 'with package_unit attribute' do
      let(:liquid_med) { create(:medication, name: 'Vitamin D 3000IU/ml Solution', dosage_unit: 'ml', dosage_amount: nil) }
      let(:scope) { Medication.where(id: liquid_med.id) }

      it 'passes package_unit to the candidate for form matching' do
        result = resolver.call(barcode: nil, name: 'Vitamin D 3000IU/ml Solution', package_unit: 'ml')
        expect(result).to eq(liquid_med)
      end
    end
  end
end
