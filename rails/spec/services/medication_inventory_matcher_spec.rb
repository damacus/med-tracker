# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationInventoryMatcher do
  subject(:matcher) { described_class.new(scope: scope) }

  # Helper to build a medication-like struct with only the attributes the matcher uses
  def med(name:, barcode: nil, dmd_code: nil, dose_unit: nil, dose_amount: nil)
    instance_double(
      Medication,
      name: name,
      barcode: barcode,
      dmd_code: dmd_code,
      dose_unit: dose_unit.to_s,
      dose_amount: dose_amount
    )
  end

  let(:paracetamol_500mg) do
    create(:medication, name: 'Paracetamol 500 mg Tablets', dose_unit: 'tablet', dose_amount: nil)
  end
  let(:paracetamol_250mg) do
    create(:medication, name: 'Paracetamol 250 mg Tablets', dose_unit: 'tablet', dose_amount: nil)
  end
  let(:amoxicillin) do
    create(:medication, name: 'Amoxicillin 500 mg Capsules', dose_unit: 'capsule', dose_amount: nil)
  end
  let(:vitamin_d_liquid) do
    create(:medication, name: 'Vitamin D Oral Solution 3000IU/ml', dose_unit: 'ml', dose_amount: nil)
  end

  let(:scope) do
    Medication.where(id: [paracetamol_500mg, paracetamol_250mg, amoxicillin, vitamin_d_liquid].map(&:id))
  end

  describe '#call' do
    context 'when matching by exact barcode' do
      before { paracetamol_500mg.update!(barcode: '5000168511017') }

      it 'returns the medication matching by barcode' do
        candidate = med(name: 'Anything', barcode: '5000168511017')
        expect(matcher.call(candidate)).to eq(paracetamol_500mg)
      end

      it 'falls through to name match when barcode does not match' do
        candidate = med(name: 'Paracetamol 500 mg Tablets', barcode: '9999999999999')
        # barcode miss → name/strength compatible match succeeds
        expect(matcher.call(candidate)).to eq(paracetamol_500mg)
      end
    end

    context 'when matching by exact dmd_code' do
      before { paracetamol_500mg.update!(dmd_code: 'DMD001', dmd_system: 'snomed') }

      it 'returns the medication matching by dmd_code' do
        candidate = med(name: 'Paracetamol 500 mg Tablets', dmd_code: 'DMD001')
        expect(matcher.call(candidate)).to eq(paracetamol_500mg)
      end
    end

    context 'when matching by compatible name' do
      it 'matches paracetamol by name and strength' do
        candidate = med(name: 'Paracetamol 500 mg Tablets')
        expect(matcher.call(candidate)).to eq(paracetamol_500mg)
      end

      it 'does not confuse 500 mg and 250 mg paracetamol' do
        candidate = med(name: 'Paracetamol 250 mg Tablets')
        expect(matcher.call(candidate)).to eq(paracetamol_250mg)
      end

      it 'matches by name key even when case differs' do
        candidate = med(name: 'PARACETAMOL 500 MG TABLETS')
        expect(matcher.call(candidate)).to eq(paracetamol_500mg)
      end

      it 'returns nil when no compatible medication found' do
        candidate = med(name: 'Ibuprofen 400 mg Tablets')
        expect(matcher.call(candidate)).to be_nil
      end

      it 'does not match across different drug names' do
        candidate = med(name: 'Amoxicillin 500 mg Tablets')
        # Amoxicillin is in scope as capsule, candidate is tablet — forms differ
        expect(matcher.call(candidate)).to be_nil
      end

      it 'matches liquid formulation by name' do
        candidate = med(name: 'Vitamin D Oral Solution 3000IU/ml', dose_unit: 'ml')
        expect(matcher.call(candidate)).to eq(vitamin_d_liquid)
      end
    end

    context 'when candidate has no barcode, dmd_code, or recognizable name' do
      it 'returns nil' do
        candidate = med(name: '')
        expect(matcher.call(candidate)).to be_nil
      end
    end

    context 'when scope is empty' do
      let(:scope) { Medication.none }

      it 'returns nil' do
        candidate = med(name: 'Paracetamol 500 mg Tablets')
        expect(matcher.call(candidate)).to be_nil
      end
    end
  end

  it 'normalises "micrograms" to "mcg" when matching strength' do
    mcg_med = create(:medication, name: 'Levothyroxine 25 micrograms Tablets',
                                  dose_unit: 'tablet', dose_amount: nil)
    scoped_matcher = described_class.new(scope: Medication.where(id: mcg_med.id))
    candidate = med(name: 'Levothyroxine 25mcg Tablets')
    expect(scoped_matcher.call(candidate)).to eq(mcg_med)
  end

  it 'returns the tablet form when candidate specifies tablet' do
    tablet_med = create(:medication, name: 'Ibuprofen 400 mg', dose_unit: 'tablet', dose_amount: nil)
    capsule_med = create(:medication, name: 'Ibuprofen 400 mg', dose_unit: 'capsule', dose_amount: nil)
    scoped_matcher = described_class.new(scope: Medication.where(id: [tablet_med, capsule_med].map(&:id)))
    candidate = med(name: 'Ibuprofen 400 mg Tablets')
    expect(scoped_matcher.call(candidate)).to eq(tablet_med)
  end

  it 'returns the capsule form when candidate specifies capsule' do
    tablet_med = create(:medication, name: 'Ibuprofen 400 mg', dose_unit: 'tablet', dose_amount: nil)
    capsule_med = create(:medication, name: 'Ibuprofen 400 mg', dose_unit: 'capsule', dose_amount: nil)
    scoped_matcher = described_class.new(scope: Medication.where(id: [tablet_med, capsule_med].map(&:id)))
    candidate = med(name: 'Ibuprofen 400 mg Capsules')
    expect(scoped_matcher.call(candidate)).to eq(capsule_med)
  end
end
