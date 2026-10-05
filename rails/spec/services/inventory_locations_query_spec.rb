# frozen_string_literal: true

require 'rails_helper'

RSpec.describe InventoryLocationsQuery do
  describe '#call' do
    it 'returns distinct locations for medications in the passed scope ordered by name' do
      alpha_location = create(:location, name: 'Alpha')
      beta_location = create(:location, name: 'Beta')
      gamma_location = create(:location, name: 'Gamma')

      first_alpha_medication = create(:medication, location: alpha_location)
      second_alpha_medication = create(:medication, location: alpha_location)
      beta_medication = create(:medication, location: beta_location)
      create(:medication, location: gamma_location)
      medication_ids = [beta_medication.id, second_alpha_medication.id, first_alpha_medication.id]

      result = described_class.new(
        medications_scope: Medication.where(id: medication_ids).includes(:location)
      ).call

      expect(result).to eq([alpha_location, beta_location])
    end

    it 'respects the passed medication scope boundary' do
      included_location = create(:location)
      excluded_location = create(:location)
      included_medication = create(:medication, location: included_location)
      create(:medication, location: excluded_location)

      result = described_class.new(
        medications_scope: Medication.where(id: included_medication.id)
      ).call

      expect(result).to contain_exactly(included_location)
    end
  end
end
