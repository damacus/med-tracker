require 'rails_helper'

RSpec.describe Reports::DoseOccurrenceQuery do
  let(:person) { create(:person) }
  let(:date) { Date.current - 40.days }
  let(:source) do
    create(:schedule, person: person, start_date: date, end_date: Date.current + 20.days, frequency: 'Daily',
                      schedule_config: { times: ['08:00'] }, max_daily_doses: 1)
  end

  def query(first: date, last: Date.current)
    described_class.new(people: [person], start_date: first, end_date: last).call
  end

  it 'projects a report range longer than one mobile occurrence page' do
    source

    expect(query.size).to eq(41)
    expect(query.map(&:window_starts_on)).to eq((date..Date.current).to_a)
  end

  it 'retains the saved outcome when a source is retired and its recurrence changes' do
    saved = record_not_taken
    source.update!(active: false, retired_at: Time.current, start_date: Date.tomorrow)

    expect(query.sole).to have_attributes(record: saved, outcome: 'not_taken', expected: false)
  end

  it 'excludes legacy as-needed representations from expected occurrences' do
    source.update!(frequency: 'As needed')

    expect(query).to be_empty
  end

  it 'does not include another persons records' do
    other = create(:person)
    create(:schedule, person: other, start_date: date, end_date: Date.current)

    expect(query).to be_empty
  end

  it 'rejects ranges outside the existing report limit' do
    expect { query(first: Date.current - 181.days) }.to raise_error(Reports::DateRange::RangeTooLarge)
  end

  it 'keeps database query counts stable as the number of schedules grows' do
    source
    baseline = count_queries { query }
    create(:schedule, person: person, start_date: date, end_date: Date.current, frequency: 'Daily')

    expect(count_queries { query }).to eq(baseline)
  end

  it 'keeps database query counts stable across mixed source kinds' do
    source
    create(:person_medication, :routine, person: person, created_at: date.in_time_zone)
    baseline = count_queries { query }
    create(:person_medication, :routine, person: person, dose_cycle: :monthly, created_at: date.in_time_zone)

    expect(count_queries { query }).to eq(baseline)
  end

  def count_queries(&)
    count = 0
    subscriber = lambda do |*arguments|
      payload = arguments.last
      count += 1 if payload[:name] != 'SCHEMA' && payload[:sql].start_with?('SELECT')
    end
    ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
    count
  end

  def record_not_taken
    account = Account.create!(email: 'history-outcome@example.test', status: :verified)
    actor = source.household.household_memberships.create!(account: account, role: :member, status: :active)
    source.medication_dose_occurrences.create!(
      window_starts_on: date, position: 1, outcome: 'not_taken', reason: 'unwell', note: 'Feeling unwell',
      resolved_at: Time.current, resolved_by_membership: actor
    )
  end
end
