# frozen_string_literal: true

require 'rails_helper'

RSpec.describe FamilyDashboard::AdministrationSourceLoader do
  fixtures :accounts, :people, :locations, :medications, :dosages, :schedules, :person_medications

  let(:person) { people(:jane) }

  it 'loads schedule and direct sources active at a historical reference time' do
    retirement_time = Time.zone.parse('2026-07-18 09:00:00')
    schedule = create_source(Schedule, retired_at: retirement_time)
    direct = create_source(PersonMedication, retired_at: retirement_time)

    result = described_class.new(
      people: [person],
      date: Date.new(2026, 7, 17),
      reference_time: Time.zone.parse('2026-07-17 23:59:59'),
      include_paused: true
    ).call

    expect(result.schedules_by_person.fetch(person.id)).to include(schedule)
    expect(result.person_medications_by_person.fetch(person.id)).to include(direct)
  end

  it 'excludes schedule and direct sources retired before a future reference time' do
    retired_schedule = create_source(Schedule, retired_at: Time.zone.parse('2026-07-19 09:00:00'))
    visible_schedule = create_source(Schedule, retired_at: Time.zone.parse('2026-07-21 09:00:00'))
    retired_direct = create_source(PersonMedication, retired_at: Time.zone.parse('2026-07-19 09:00:00'))
    visible_direct = create_source(PersonMedication, retired_at: Time.zone.parse('2026-07-21 09:00:00'))

    result = described_class.new(
      people: [person],
      date: Date.new(2026, 7, 20),
      reference_time: Time.zone.parse('2026-07-20 00:00:00'),
      include_paused: true
    ).call

    expect(result.schedules_by_person.fetch(person.id)).to include(visible_schedule)
    expect(result.person_medications_by_person.fetch(person.id)).to include(visible_direct)
    expect(result.schedules_by_person.fetch(person.id)).not_to include(retired_schedule)
    expect(result.person_medications_by_person.fetch(person.id)).not_to include(retired_direct)
  end

  def create_source(source_class, retired_at:)
    medication = create(:medication, household: person.household)
    create(
      source_class.model_name.singular.to_sym,
      household: person.household,
      person: person,
      medication: medication,
      dosage: nil,
      dose_amount: medication.dose_amount,
      dose_unit: medication.dose_unit,
      retired_at: retired_at
    )
  end
end
