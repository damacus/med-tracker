# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Reports::HealthHistoryQuery do
  fixtures :people, :locations, :medications, :dosages

  let(:person) { people(:john) }
  let(:start_date) { Date.new(2026, 2, 1) }
  let(:end_date) { Date.new(2026, 2, 28) }

  it 'includes scheduled and direct medication takes in the inclusive date range' do
    schedule = create(:schedule, person: person, medication: medications(:paracetamol),
                                 dosage: dosages(:paracetamol_adult))
    direct = create(:person_medication, person: person, medication: medications(:ibuprofen),
                                        dosage: dosages(:ibuprofen_light))
    create(:medication_take, :for_schedule, schedule: schedule, taken_at: Time.zone.parse('2026-02-01 08:30'))
    create(:medication_take, :for_person_medication, person_medication: direct,
                                                     taken_at: Time.zone.parse('2026-02-28 20:45'))
    create(:medication_take, :for_schedule, schedule: schedule, taken_at: Time.zone.parse('2026-03-01 08:30'))

    result = described_class.new(people: Person.where(id: person.id), start_date: start_date, end_date: end_date).call

    expect(result.medication_takes.map(&:medication_name)).to eq(%w[Paracetamol Ibuprofen])
    expect(result.medication_takes.map(&:source_type)).to eq(%i[scheduled as_needed])
    expect(result.medication_takes.map(&:dose_display)).to eq(['1000.0 mg', '200.0 mg'])
  end

  it 'accepts an array of people and marks PRN schedule takes as as-needed' do
    schedule = create(:schedule, person: person, medication: medications(:paracetamol),
                                 dosage: dosages(:paracetamol_adult), schedule_type: :prn)
    create(:medication_take, :for_schedule, schedule: schedule, taken_at: Time.zone.parse('2026-02-10 08:30'))

    result = described_class.new(people: [person], start_date: start_date, end_date: end_date).call

    expect(result.people).to eq([person])
    expect(result.medication_takes.sole.source_type).to eq(:as_needed)
  end

  it 'includes suspected side effects, illnesses, linked medication snapshots, and illness patterns' do
    create_suspected_side_effect
    create_illness_episode(Date.new(2026, 2, 1), Date.new(2026, 2, 3), 'Cold')
    create_illness_episode(Date.new(2026, 2, 20), Date.new(2026, 2, 22), ' cold ')

    result = described_class.new(people: Person.where(id: person.id), start_date: start_date, end_date: end_date).call

    expect(result.suspected_side_effects.sole).to have_attributes(
      title: 'Nausea',
      medication_names: ['Paracetamol'],
      medical_help_sought: true
    )
    expect(result.notable_illnesses.map(&:title)).to eq(%w[Cold cold])
    expect(result.illness_patterns.sole).to have_attributes(
      normalized_title: 'cold',
      episode_count: 2,
      average_interval_days: 19
    )
  end

  def create_illness_episode(started_on, ended_on, title)
    HealthEvent.create!(
      person: person,
      event_kind: :illness,
      title: title,
      started_on: started_on,
      ended_on: ended_on
    )
  end

  def create_suspected_side_effect
    side_effect = HealthEvent.create!(
      person: person,
      event_kind: :suspected_side_effect,
      title: 'Nausea',
      started_on: Date.new(2026, 2, 10),
      severity: :moderate,
      notes: 'Started after evening dose',
      action_taken: 'Called pharmacy',
      medical_help_sought: true
    )
    HealthEventMedication.create!(health_event: side_effect, medication: medications(:paracetamol))
  end
end
