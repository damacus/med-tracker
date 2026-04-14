# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Reports::IndexQuery do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :schedules, :medication_takes

  let(:people_scope) { Person.where(id: [people(:john).id, people(:jane).id]) }
  let(:start_date) { Time.zone.today - 1.day }
  let(:end_date) { Time.zone.today }

  describe '#call' do
    it 'returns daily compliance data and inventory alerts for the passed people and date range' do
      result = described_class.new(people: people_scope, start_date: start_date, end_date: end_date).call

      expect(result.daily_data.pluck(:date)).to eq([start_date, end_date])
      expect(result.daily_data.last[:expected]).to be >= 1
      expect(result.daily_data.last[:actual]).to be >= 0
      expect(result.inventory_alerts).to all(include(:medication_name, :days_left, :doses_left, :low_stock))
    end

    it 'treats days with no expected doses as 100 percent compliance' do
      result = described_class.new(people: Person.none, start_date: start_date, end_date: end_date).call

      expect(result.daily_data).to all(include(percentage: 100, expected: 0, actual: 0))
    end

    it 'limits inventory alerts to the soonest two items under 14 days left' do
      medication_one = create(:medication, current_supply: 3, supply_at_last_restock: 3)
      medication_two = create(:medication, current_supply: 5, supply_at_last_restock: 5)
      medication_three = create(:medication, current_supply: 10, supply_at_last_restock: 10)

      create(:schedule, person: people(:john), medication: medication_one, max_daily_doses: 1)
      create(:schedule, person: people(:john), medication: medication_two, max_daily_doses: 1)
      create(:schedule, person: people(:john), medication: medication_three, max_daily_doses: 1)

      result = described_class.new(
        people: Person.where(id: people(:john).id),
        start_date: start_date,
        end_date: end_date
      ).call

      expect(result.inventory_alerts.size).to be <= 2
      expect(result.inventory_alerts.pluck(:days_left)).to eq(result.inventory_alerts.pluck(:days_left).sort)
    end
  end
end
