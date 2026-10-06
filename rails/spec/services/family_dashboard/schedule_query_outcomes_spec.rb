require 'rails_helper'

RSpec.describe FamilyDashboard::ScheduleQuery do
  fixtures :households, :accounts, :people, :locations, :medications, :dosages, :schedules

  let(:source) { schedules(:john_movicol) }
  let(:query) { described_class.new([source.person]) }

  before { travel_to Time.current.change(hour: 12, min: 0, sec: 0) }

  def record_not_taken
    account = Account.create!(email: 'dashboard-outcomes@example.test', status: :verified)
    actor = source.household.household_memberships.create!(account: account, role: :member, status: :active)
    source.medication_dose_occurrences.create!(
      window_starts_on: Date.current, position: 1, scheduled_at: Time.current.change(hour: 8),
      outcome: 'not_taken', reason: 'unwell', note: 'Feeling unwell',
      resolved_at: Time.current, resolved_by_membership: actor
    )
  end

  it 'removes a resolved occurrence from outstanding tasks without pretending it was administered' do
    record_not_taken

    expect(query.call.pluck(:source)).not_to include(source)
    expect(query.today_takes_by_person.fetch(source.person).map(&:source)).not_to include(source)
  end

  it 'retains not-taken context for the person dashboard' do
    outcome = record_not_taken

    expect(query.today_not_taken_by_person.fetch(source.person)).to contain_exactly(outcome)
  end

  it 'shows the next unresolved time without incrementing the taken dose count' do
    source.update!(schedule_type: :multiple_daily, schedule_config: { times: %w[08:00 20:00] }, max_daily_doses: 2)
    record_not_taken

    row = query.call.find { |task| task[:source] == source }

    expect(row[:scheduled_at]).to eq(Time.current.change(hour: 20))
    expect(row[:daily_dose_count]).to eq(0)
    expect(row[:not_taken_count]).to eq(1)
  end

  it 'restores an outstanding task when the outcome is reopened' do
    record_not_taken.update!(outcome: 'open', reason: nil, note: nil, resolved_at: nil, resolved_by_membership: nil)

    expect(query.call.pluck(:source)).to include(source)
  end

  it 'identifies an unresolved timed occurrence as overdue' do
    source.update!(schedule_config: { times: ['08:00'] })

    expect(query.call.find { |task| task[:source] == source }[:overdue]).to be(true)
  end
end
