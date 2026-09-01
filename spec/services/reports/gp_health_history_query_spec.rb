# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Reports::GpHealthHistoryQuery do
  fixtures :accounts, :people, :locations, :medications, :dosages

  let(:person) { people(:john) }
  let(:start_date) { Date.new(2026, 2, 1) }
  let(:end_date) { Date.new(2026, 2, 28) }

  it 'returns one person, current medicines, and a most-recent-first mixed chronology' do
    schedule = create_schedule
    create_direct_medicine
    create_illness
    create_side_effect

    result = described_class.new(person:, start_date:, end_date:).call

    expect(result.person).to eq(person)
    expect(result.current_medicines.map(&:name)).to include(schedule.medication.name, medications(:ibuprofen).name)
    expect(result.chronology.map(&:title)).to eq(%w[Nausea Cold])
    expect(result.chronology.first).to have_attributes(**side_effect_attributes)
    expect(result.chronology.last.duration_days).to eq(3)
  end

  it 'loads health events once' do
    HealthEvent.create!(person:, event_kind: :illness, title: 'Cold', started_on: start_date)

    expect(count_health_event_queries do
      described_class.new(person:, start_date:, end_date:).call
    end).to eq(1)
  end

  def count_health_event_queries(&)
    count = 0
    subscriber = lambda do |_name, _start, _finish, _id, payload|
      sql = payload[:sql]
      next if payload[:cached] || payload[:name] == 'SCHEMA'

      count += 1 if sql.include?('FROM "health_events"')
    end

    ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
    count
  end

  def create_schedule
    create(:schedule, person:, medication: medications(:paracetamol), dosage: dosages(:paracetamol_adult))
  end

  def create_direct_medicine
    create(:person_medication, person:, medication: medications(:ibuprofen), dosage: dosages(:ibuprofen_light))
  end

  def create_illness
    HealthEvent.create!(person:, event_kind: :illness, title: 'Cold', started_on: Date.new(2026, 2, 4),
                        ended_on: Date.new(2026, 2, 6))
  end

  def create_side_effect
    event = HealthEvent.create!(person:, event_kind: :suspected_side_effect, title: 'Nausea',
                                started_on: Date.new(2026, 2, 20), severity: :moderate,
                                notes: 'Started after evening dose', action_taken: 'Called pharmacy',
                                medical_help_sought: true)
    HealthEventMedication.create!(health_event: event, medication: medications(:paracetamol))
  end

  def side_effect_attributes
    { duration_days: nil, severity: 'moderate', notes: 'Started after evening dose', action_taken: 'Called pharmacy',
      medical_help_sought: true, medication_names: ['Paracetamol'] }
  end
end
