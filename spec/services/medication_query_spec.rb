# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationQuery do
  fixtures :locations, :medications

  let(:scope) { Medication.all }

  before do
    Medication.create!(
      name: 'School Only Medicine',
      location: locations(:school),
      category: 'Analgesic',
      dose_amount: 500,
      dose_unit: 'mg',
      current_supply: 10,
      reorder_threshold: 1
    )
  end

  it 'returns all medications when no filters are provided' do
    results = described_class.new(scope: scope).call

    expect(results.map(&:name)).to include('Paracetamol', 'Vitamin D', 'School Only Medicine')
    expect(results.first.association(:location)).to be_loaded
  end

  it 'filters by category' do
    results = described_class.new(scope: scope, category: 'Vitamin').call

    expect(results.map(&:name)).to contain_exactly('Vitamin C', 'Vitamin D')
  end

  it 'calculates consumption for the loaded list without per-medication queries' do
    schedule = create(:schedule, max_daily_doses: 2)
    direct = create(:person_medication, max_daily_doses: 3)
    results = described_class.new(scope: scope).call.to_a
    statements = []
    subscriber = lambda do |_name, _start, _finish, _id, payload|
      statements << payload[:sql] unless payload[:cached] || payload[:name] == 'SCHEMA'
    end

    rates = ActiveRecord::Base.uncached do
      ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record') do
        results.to_h { |medication| [medication.id, MedicationDailyConsumption.new(medication).call] }
      end
    end

    expect(rates.fetch(schedule.medication_id)).to eq(2.0)
    expect(rates.fetch(direct.medication_id)).to eq(3.0)
    expect(statements).to be_empty
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
      dose_amount: 250,
      dose_unit: 'mg',
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
