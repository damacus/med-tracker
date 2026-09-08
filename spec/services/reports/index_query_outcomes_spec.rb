require 'rails_helper'

RSpec.describe Reports::IndexQuery do
  let(:person) { create(:person) }
  let(:date) { Date.yesterday }
  let(:source) do
    create(:schedule, person: person, start_date: date, end_date: Date.tomorrow, frequency: 'Daily',
                      schedule_type: :multiple_daily, schedule_config: { times: %w[08:00 20:00] }, max_daily_doses: 2)
  end

  def report_row(on: date)
    described_class.new(people: [person], start_date: on, end_date: on).call.daily_data.sole
  end

  def record_not_taken(position: 1)
    account = Account.create!(email: "report-outcomes-#{position}@example.test", status: :verified)
    actor = source.household.household_memberships.create!(account: account, role: :member, status: :active)
    source.medication_dose_occurrences.create!(
      window_starts_on: date, position: position, outcome: 'not_taken', reason: 'unwell',
      resolved_at: Time.current, resolved_by_membership: actor
    )
  end

  it 'separates not-taken from actual administrations and unexplained misses' do
    record_not_taken

    expect(report_row).to include(expected: 2, actual: 0, not_taken: 1, unexplained_missed: 1, percentage: 0)
  end

  it 'does not classify fully explained non-administration as an unexplained miss' do
    record_not_taken
    record_not_taken(position: 2)

    expect(report_row).to include(actual: 0, not_taken: 2, unexplained_missed: 0, percentage: 0)
  end

  it 'does not classify future occurrences as missed' do
    source

    expect(report_row(on: Date.tomorrow)).to include(expected: 2, unexplained_missed: 0)
  end

  it 'counts only elapsed timed occurrences as unexplained today' do
    source
    travel_to Time.current.change(hour: 12, min: 0, sec: 0) do
      expect(report_row(on: Date.current)).to include(expected: 2, unexplained_missed: 1)
    end
  end

  it 'never treats an as-needed source as an unexplained scheduled miss' do
    source.update!(schedule_type: :prn)

    travel_to Time.current.change(hour: 12, min: 0, sec: 0) do
      expect(report_row(on: Date.current)).to include(expected: 0, unexplained_missed: 0)
    end
  end

  it 'does not allow extra takes of another source to hide an unexplained miss' do
    source
    other = create(:schedule, person: person, start_date: date, end_date: date, max_daily_doses: 3)
    3.times { create(:medication_take, :for_schedule, schedule: other, taken_at: date.in_time_zone + 12.hours) }

    expect(report_row[:unexplained_missed]).to be >= 2
  end

  it 'does not count a reopened outcome as not taken' do
    record_not_taken.update!(outcome: 'open', reason: nil, resolved_at: nil, resolved_by_membership: nil)

    expect(report_row).to include(not_taken: 0, unexplained_missed: 2)
  end

  it 'retains resolved history after a schedule is retired' do
    record_not_taken
    source.update!(active: false, retired_at: Time.current)

    expect(report_row).to include(expected: 1, not_taken: 1, unexplained_missed: 0)
  end

  it 'does not let an old resolved position hide a remaining open position after an edit' do
    record_not_taken(position: 2)
    source.update!(schedule_config: { times: ['08:00'] })

    expect(report_row).to include(expected: 2, not_taken: 1, unexplained_missed: 1)
  end

  it 'excludes legacy as-needed source expectations' do
    source.update!(frequency: 'As needed')

    expect(report_row).to include(expected: 0, unexplained_missed: 0)
  end
end
