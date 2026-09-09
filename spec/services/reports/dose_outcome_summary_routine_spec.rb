require 'rails_helper'

RSpec.describe Reports::DoseOutcomeSummary do
  let(:person) { create(:person) }
  let(:source) do
    create(:person_medication, :routine, person: person, dose_cycle: :monthly,
                                         max_daily_doses: 2, created_at: 3.months.ago)
  end

  before { travel_to(Time.zone.local(2026, 9, 9, 12)) }

  def summary(first: Date.current.beginning_of_month, last: Date.current)
    described_class.new(people: [person], start_date: first, end_date: last)
  end

  it 'keeps unfinished monthly positions out of daily adherence and missed counts' do
    source
    result = summary

    expect(result.for_date(Date.current.beginning_of_month)).to include(expected: 0, unexplained_missed: 0)
    expect(result.cycle_summaries.sole).to include(source: source, expected: 2, actual: 0,
                                                   not_taken: 0, unexplained_missed: 0,
                                                   window_ends_on: Date.current.end_of_month)
  end

  it 'allocates a monthly take once across report pages and keeps its actual timestamp' do
    take = create(:medication_take, person_medication: source, taken_at: Time.current)
    query = Reports::DoseOccurrenceQuery.new(people: [person], start_date: Date.new(2026, 8, 1),
                                             end_date: Date.new(2026, 9, 30))

    expect(query.call.size).to eq(4)
    expect(query.call.count { |row| row.outcome == 'taken' }).to eq(1)
    expect(query.takes.sole).to eq(take)
    expect(summary.for_date(Date.current)).to include(expected: 0, actual: 0)
    expect(summary.cycle_summaries.sole).to include(actual: 1, unexplained_missed: 0)
  end

  it 'marks only unresolved positions in completed cycles as unexplained' do
    source
    result = summary(first: Date.new(2026, 8, 10), last: Date.new(2026, 8, 20))

    expect(result.cycle_summaries.sole).to include(expected: 2, unexplained_missed: 2)
  end

  it 'uses the existing weekly window and waits until it ends to count misses' do
    source.update!(dose_cycle: :weekly)

    expect(summary(first: Date.current, last: Date.current).cycle_summaries.sole).to include(
      window_starts_on: Date.current.beginning_of_week, window_ends_on: Date.current.end_of_week,
      unexplained_missed: 0
    )
  end

  it 'retains explicit not-taken context after a routine source is retired' do
    account = Account.create!(email: 'routine-report@example.test', status: :verified)
    actor = source.household.household_memberships.create!(account: account, role: :member, status: :active)
    outcome = source.medication_dose_occurrences.create!(
      window_starts_on: Date.current.beginning_of_month, position: 1, outcome: 'not_taken',
      reason: 'unwell', resolved_at: Time.current, resolved_by_membership: actor
    )
    source.retire!
    source.update!(dose_cycle: :daily)

    result = summary(first: Date.current, last: Date.current)
    expect(result.not_taken_outcomes.sole.record).to eq(outcome)
    expect(result.cycle_summaries.sole).to include(not_taken: 1, actual: 0, unexplained_missed: 0)
  end

  it 'includes daily routine expectations and actual administrations in daily counts' do
    source.update!(dose_cycle: :daily)
    create(:medication_take, person_medication: source, taken_at: Time.current)

    expect(summary.for_date(Date.current)).to include(expected: 2, actual: 1, unexplained_missed: 0)
    expect(summary.cycle_summaries).to be_empty
  end

  it 'excludes as-needed assignments and people outside the requested scope' do
    source.update!(administration_kind: :as_needed)
    create(:person_medication, :routine, dose_cycle: :monthly, created_at: 3.months.ago)

    expect(summary.cycle_summaries).to be_empty
    expect(summary.for_date(Date.current)).to include(expected: 0, actual: 0)
  end
end
