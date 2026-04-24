# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Reports::IndexQuery do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :schedules, :medication_takes

  let(:people_scope) { Person.where(id: [people(:john).id, people(:jane).id]) }
  let(:start_date) { Time.zone.today - 1.day }
  let(:end_date) { Time.zone.today }

  def create_report_schedule(person:, medication:, report_date:, schedule_type:, schedule_config: {})
    create(
      :schedule,
      person: person,
      medication: medication,
      schedule_type: schedule_type,
      schedule_config: schedule_config,
      start_date: report_date,
      end_date: report_date,
      max_daily_doses: 4
    )
  end

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

    it 'uses schedule expected doses for compliance calculations' do
      report_date = Date.new(2026, 4, 21)
      medication = create(:medication)
      create_report_schedule(person: people(:john), medication: medication, report_date: report_date,
                             schedule_type: :prn)
      create_report_schedule(person: people(:john), medication: medication, report_date: report_date,
                             schedule_type: :multiple_daily, schedule_config: { 'times' => %w[08:00 20:00] })

      result = described_class.new(
        people: Person.where(id: people(:john).id),
        start_date: report_date,
        end_date: report_date
      ).call

      expect(result.daily_data.first).to include(expected: 2, actual: 0, percentage: 0)
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
