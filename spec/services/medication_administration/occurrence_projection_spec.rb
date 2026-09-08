require 'rails_helper'

RSpec.describe MedicationAdministration::OccurrenceProjection do
  fixtures :households, :accounts, :people, :locations, :medications, :dosages, :schedules

  let(:schedule) { schedules(:john_movicol) }
  let(:date) { Date.current }
  let(:membership) do
    schedule.household.household_memberships.find_or_create_by!(account: accounts(:admin)) do |record|
      record.role = :owner
      record.status = :active
    end
  end

  def project(first: date, last: date)
    described_class.new(source: schedule.reload, start_date: first, end_date: last).call
  end

  def pause(from:, to: nil)
    schedule.medication_pause_periods.create!(
      reason: 'clinician_advice', started_at: from, ended_at: to,
      recorded_by_membership: membership, resumed_by_membership: to ? membership : nil
    )
  end

  it 'derives stable untimed identities without writing rows' do
    rows = nil
    expect { rows = project }.not_to change(MedicationDoseOccurrence, :count)

    expect(rows.size).to eq(1)
    expect(rows.first).to have_attributes(position: 1, scheduled_at: nil, outcome: 'open')
    expect(project.map(&:key)).to eq(rows.map(&:key))
  end

  it 'preserves original ordinals when a partial pause excludes a timed occurrence' do
    schedule.update!(schedule_config: { times: %w[08:00 12:00 20:00] })
    pause(from: date.in_time_zone.change(hour: 11), to: date.in_time_zone.change(hour: 13))

    expect(project.map(&:position)).to eq([1, 3])
    expect(project.map { |row| row.scheduled_at.hour }).to eq([8, 20])
  end

  it 'keeps an untimed window during a partial-day pause' do
    pause(from: date.in_time_zone.change(hour: 11), to: date.in_time_zone.change(hour: 13))

    expect(project.size).to eq(1)
  end

  it 'excludes an untimed window covered by contiguous pauses' do
    pause(from: date.in_time_zone, to: date.in_time_zone.change(hour: 12))
    pause(from: date.in_time_zone.change(hour: 12))

    expect(project).to be_empty
  end

  it 'excludes every retained as-needed representation' do
    schedule.update!(schedule_type: :prn)
    expect(project).to be_empty
    schedule.update!(schedule_type: :daily, frequency: 'As needed')
    expect(project).to be_empty
    schedule.update!(frequency: 'Daily', schedule_config: { as_needed: true })
    expect(project).to be_empty
  end

  it 'respects schedule range and specific-date recurrence' do
    schedule.update!(schedule_type: :specific_dates, schedule_config: { dates: [(date + 1).iso8601] })

    expect(project(first: date, last: date + 2).map(&:window_starts_on)).to eq([date + 1])
    expect(project(first: date - 2, last: date - 1)).to be_empty
  end

  it 'does not invent occurrences on or after retirement' do
    schedule.update!(retired_at: (date + 1).in_time_zone)

    expect(project(first: date, last: date + 2).map(&:window_starts_on)).to eq([date])
  end

  it 'distinguishes future timed occurrences from due ones' do
    schedule.update!(schedule_config: { times: %w[08:00 20:00] })
    travel_to(date.in_time_zone.change(hour: 12)) do
      expect(project.map(&:due?)).to eq([true, false])
    end
  end

  it 'preserves resolved snapshots after a schedule edit removes the expectation' do
    saved = schedule.medication_dose_occurrences.create!(
      window_starts_on: date, position: 1, outcome: 'not_taken', reason: 'unwell',
      resolved_at: Time.current, resolved_by_membership: membership
    )
    schedule.update!(start_date: date + 1)

    expect(project.first).to have_attributes(record: saved, outcome: 'not_taken')
  end

  it 'rejects missing reversed and overlong date ranges' do
    expect { project(first: nil) }.to raise_error(ArgumentError)
    expect { project(last: date - 1) }.to raise_error(ArgumentError)
    expect { project(last: date + 31) }.to raise_error(ArgumentError)
  end

  it 'resolves only authentic opaque keys' do
    row = project.first

    expect(described_class.decode(row.key)).to eq(['schedule', schedule.portable_id, date.iso8601, 1])
    expect(described_class.decode("#{row.key}tampered")).to be_nil
  end
end
