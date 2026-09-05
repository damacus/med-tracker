# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::ScheduleSerializer do
  it 'serialises the recurrence configuration without losing configured dates or times' do
    config = { 'dates' => ['2026-09-08'], 'times' => ['08:00'] }
    schedule = build_stubbed(:schedule, schedule_type: :specific_dates, schedule_config: config)

    expect(described_class.new(schedule).as_json).to include(
      schedule_type: 'specific_dates', schedule_config: config
    )
  end

  it 'serialises association, timing and dosing data' do
    schedule = create(:schedule)
    json = described_class.new(schedule).as_json
    expect(json).to include(
      id: schedule.id, portable_id: schedule.portable_id, person_id: schedule.person_id,
      person_portable_id: schedule.person.portable_id, medication_id: schedule.medication_id,
      medication_portable_id: schedule.medication.portable_id,
      frequency: schedule.frequency, dose_cycle: schedule.dose_cycle, active: schedule.active?, paused: false,
      max_daily_doses: schedule.max_daily_doses,
      min_hours_between_doses: BigDecimal(schedule.min_hours_between_doses.to_s).to_s('F')
    )
    expect(json[:start_date]).to eq(schedule.start_date&.iso8601)
    expect(json[:end_date]).to eq(schedule.end_date&.iso8601)
    expect(json[:updated_at]).to eq(schedule.updated_at.iso8601)
  end

  it 'includes dose_amount, dose_unit and notes' do
    schedule = create(:schedule, notes: 'Take with food')
    json = described_class.new(schedule).as_json
    expect(json).to include(
      dose_amount: BigDecimal(schedule.dose_amount.to_s).to_s('F'),
      dose_unit: schedule.dose_unit,
      notes: 'Take with food'
    )
  end

  it 'serialises nil start_date and end_date as nil' do
    schedule = build_stubbed(:schedule, start_date: nil, end_date: nil)
    json = described_class.new(schedule).as_json
    expect(json[:start_date]).to be_nil
    expect(json[:end_date]).to be_nil
  end

  it 'serialises an expired (inactive) schedule with active: false' do
    schedule = create(:schedule, :expired)
    json = described_class.new(schedule).as_json
    expect(json[:active]).to be false
  end

  it 'serialises paused schedules with paused true and active false' do
    schedule = create(:schedule, active: false)
    json = described_class.new(schedule).as_json

    expect(json[:active]).to be false
    expect(json[:paused]).to be true
  end

  it 'serialises missing optional associations as nil portable IDs' do
    updated_at = Time.zone.parse('2026-07-07 12:00:00')
    json = described_class.new(missing_association_schedule(updated_at)).as_json

    expect(json).to include(
      person_portable_id: nil,
      medication_portable_id: nil,
      start_date: nil,
      end_date: nil
    )
  end

  def missing_association_schedule(updated_at)
    instance_double(
      Schedule,
      id: 1, portable_id: SecureRandom.uuid, person_id: nil, person: nil,
      medication_id: nil, medication: nil, dose_amount: nil, dose_unit: nil,
      frequency: nil, dose_cycle: 'daily', start_date: nil, end_date: nil,
      active?: false, paused?: true, notes: nil, updated_at: updated_at,
      max_daily_doses: nil, min_hours_between_doses: nil, schedule_type: 'daily', schedule_config: {}
    )
  end
end
