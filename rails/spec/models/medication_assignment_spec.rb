# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationAssignment do
  it 'casts attributes to their declared types' do
    assignment = described_class.new(
      medication_id: '5', source_dosage_option_id: '7', dose_amount: '2.5', dose_unit: 'ml'
    )
    expect(assignment).to have_attributes(
      medication_id: 5, source_dosage_option_id: 7, dose_amount: BigDecimal('2.5'), dose_unit: 'ml'
    )
  end
end
