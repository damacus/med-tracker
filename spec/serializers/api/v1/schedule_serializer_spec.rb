# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::ScheduleSerializer do
  it 'serialises association, timing and dosing data' do
    schedule = create(:schedule)
    json = described_class.new(schedule).as_json
    expect(json).to include(
      id: schedule.id, person_id: schedule.person_id, medication_id: schedule.medication_id,
      frequency: schedule.frequency, dose_cycle: schedule.dose_cycle, active: schedule.active?, paused: false,
      max_daily_doses: schedule.max_daily_doses, min_hours_between_doses: schedule.min_hours_between_doses
    )
    expect(json[:start_date]).to eq(schedule.start_date&.iso8601)
    expect(json[:end_date]).to eq(schedule.end_date&.iso8601)
    expect(json[:updated_at]).to eq(schedule.updated_at.iso8601)
  end

  it 'includes dose_amount, dose_unit and notes' do
    schedule = create(:schedule, notes: 'Take with food')
    json = described_class.new(schedule).as_json
    expect(json).to include(
      dose_amount: schedule.dose_amount,
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
end
