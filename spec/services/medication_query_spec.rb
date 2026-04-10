# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationQuery do
  fixtures :locations, :medications

  let(:scope) { Medication.includes(:location) }

  before do
    Medication.create!(
      name: 'School Only Medicine',
      location: locations(:school),
      category: 'Analgesic',
      dosage_amount: 500,
      dosage_unit: 'mg',
      current_supply: 10,
      reorder_threshold: 1
    )
  end

  it 'returns all medications when no filters are provided' do
    results = described_class.new(scope: scope).call

    expect(results.map(&:name)).to include('Paracetamol', 'Vitamin D', 'School Only Medicine')
  end

  it 'filters by category' do
    results = described_class.new(scope: scope, category: 'Vitamin').call

    expect(results.map(&:name)).to contain_exactly('Vitamin C', 'Vitamin D')
  end

  it 'filters by location' do
    results = described_class.new(scope: scope, location_id: locations(:school).id).call

    expect(results.map(&:name)).to contain_exactly('School Only Medicine')
  end

  it 'combines category and location filters' do
    school_vitamin = Medication.create!(
      name: 'School Vitamin',
      location: locations(:school),
      category: 'Vitamin',
      dosage_amount: 250,
      dosage_unit: 'mg',
      current_supply: 12,
      reorder_threshold: 2
    )

    results = described_class.new(
      scope: scope,
      category: 'Vitamin',
      location_id: locations(:school).id
    ).call

    expect(results).to contain_exactly(school_vitamin)
  end

  it 'returns categories for the filtered scope in sorted order' do
    categories = described_class.new(scope: scope, location_id: locations(:school).id).categories

    expect(categories).to eq(%w[Analgesic])
  end
end
