require 'rails_helper'

RSpec.describe Reports::HealthHistoryQuery do
  let(:person) { create(:person) }
  let(:date) { Date.yesterday }
  let(:source) do
    create(:schedule, person: person, start_date: date, end_date: Date.current,
                      frequency: 'Daily', max_daily_doses: 1)
  end

  def report(people: [person])
    described_class.new(people: people, start_date: date, end_date: date).call
  end

  def record_not_taken
    account = Account.create!(email: 'report-history-outcome@example.test', status: :verified)
    actor = source.household.household_memberships.create!(account: account, role: :member, status: :active)
    source.medication_dose_occurrences.create!(
      window_starts_on: date, position: 1, outcome: 'not_taken', reason: 'unwell', note: 'Feeling unwell',
      resolved_at: Time.current, resolved_by_membership: actor
    )
  end

  it 'provides separate non-administration context and daily outcome counts' do
    record_not_taken
    result = report

    expect(result.not_taken_outcomes.sole).to have_attributes(person: person, date: date, reason: 'unwell',
                                                              note: 'Feeling unwell')
    expect(result.medication_takes).to be_empty
    expect(result.daily_outcomes.sole).to include(expected: 1, actual: 0, not_taken: 1, unexplained_missed: 0)
  end

  it 'keeps the recorded context after source retirement' do
    record_not_taken
    source.retire!

    expect(report.not_taken_outcomes.sole.note).to eq('Feeling unwell')
  end

  it 'omits another persons outcome history' do
    record_not_taken

    expect(report(people: []).not_taken_outcomes).to be_empty
  end

  it 'omits current not-taken context after the outcome is reopened' do
    record_not_taken.update!(outcome: 'open', reason: nil, note: nil, resolved_at: nil, resolved_by_membership: nil)

    expect(report.not_taken_outcomes).to be_empty
    expect(report.daily_outcomes.sole).to include(not_taken: 0, unexplained_missed: 1)
  end
end
