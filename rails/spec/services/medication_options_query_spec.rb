# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationOptionsQuery do
  describe '#call' do
    it 'returns medications in name order' do
      zeta_medication = create(:medication, name: 'Zeta Medication')
      alpha_medication = create(:medication, name: 'Alpha Medication')

      result = described_class.new(
        scope: Medication.where(id: [zeta_medication.id, alpha_medication.id])
      ).call

      expect(result.map(&:name)).to eq(['Alpha Medication', 'Zeta Medication'])
    end

    it 'respects the passed scope boundary' do
      included_medication = create(:medication)
      create(:medication)

      result = described_class.new(scope: Medication.where(id: included_medication.id)).call

      expect(result).to contain_exactly(included_medication)
    end
  end

  describe '#include?' do
    it 'returns true only for medication ids inside the passed scope' do
      included_medication = create(:medication)
      excluded_medication = create(:medication)

      query = described_class.new(scope: Medication.where(id: included_medication.id))

      expect(query).to include(included_medication.id)
      expect(query).not_to include(excluded_medication.id)
    end
  end
end
