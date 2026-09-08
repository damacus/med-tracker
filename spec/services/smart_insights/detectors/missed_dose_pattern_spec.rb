# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::Detectors::MissedDosePattern do
  fixtures :accounts, :people

  def context_with(daily_data) = instance_double(SmartInsights::Context, daily_data: daily_data)

  def day(expected:, actual:)
    { expected: expected, actual: actual, unexplained_missed: [expected - actual, 0].max }
  end

  it 'does not turn consecutive explained not-taken outcomes into a missed-dose warning' do
    explained = day(expected: 1, actual: 0).merge(not_taken: 1, unexplained_missed: 0)

    expect(described_class.new(context_with([explained, explained])).call).to be_empty
  end

  it 'breaks an unexplained streak when an intervening day is explained' do
    missed = day(expected: 1, actual: 0)
    explained = missed.merge(not_taken: 1, unexplained_missed: 0)

    expect(described_class.new(context_with([missed, explained, missed])).call).to be_empty
  end

  it 'stays silent when the longest missed streak is 1' do
    data = [day(expected: 1, actual: 0), day(expected: 1, actual: 1), day(expected: 1, actual: 0)]
    expect(described_class.new(context_with(data)).call).to eq([])
  end

  it 'warns (key/family/severity) when two consecutive days are missed' do
    data = [day(expected: 1, actual: 0), day(expected: 1, actual: 0), day(expected: 1, actual: 1)]
    insight = described_class.new(context_with(data)).call.first
    expect(insight).to have_attributes(key: :missed_dose_pattern, family: :adherence, severity: :warning)
  end

  it 'sets correct I18n fields on the missed_dose_pattern insight' do
    data = [day(expected: 1, actual: 0), day(expected: 1, actual: 0), day(expected: 1, actual: 1)]
    insight = described_class.new(context_with(data)).call.first
    expect(insight.title).to eq(I18n.t('smart_insights.detectors.missed_dose_pattern.title'))
    expect(insight.summary).to eq(I18n.t('smart_insights.detectors.missed_dose_pattern.summary', count: 2))
    expect(insight.detail).to eq(I18n.t('smart_insights.detectors.missed_dose_pattern.detail'))
    expect(insight.metric_label).to eq(I18n.t('smart_insights.detectors.missed_dose_pattern.metric_label'))
    expect(insight.metric_value).to eq(I18n.t('smart_insights.detectors.missed_dose_pattern.metric_value', count: 2))
  end

  it 'does not count a day with zero expected as missed' do
    data = [day(expected: 0, actual: 0), day(expected: 0, actual: 0)]
    expect(described_class.new(context_with(data)).call).to eq([])
  end

  it 'tracks the longest streak, not the most recent' do
    data = [
      day(expected: 1, actual: 0), day(expected: 1, actual: 0),
      day(expected: 1, actual: 1), day(expected: 1, actual: 0)
    ]
    expect(described_class.new(context_with(data)).call.size).to eq(1)
  end

  it 'counts resumed misses without counting completed pause days' do
    start_date = Date.new(2026, 4, 20)
    person = create(:person)
    schedule = create(:schedule, person: person, start_date: start_date, end_date: start_date + 3.days,
                                 schedule_type: :multiple_daily, schedule_config: { 'times' => ['08:00'] },
                                 max_daily_doses: 1, frequency: 'Daily')
    record_pause(schedule, started_at: start_date.in_time_zone + 8.hours,
                           ended_at: (start_date + 2.days).in_time_zone + 8.hours)
    context = SmartInsights::Context.new(people: [person], start_date: start_date, end_date: start_date + 3.days)

    insight = described_class.new(context).call.sole

    expect(insight.metric_value).to eq(I18n.t('smart_insights.detectors.missed_dose_pattern.metric_value', count: 2))
  end

  it 'uses persisted explanations without claiming an administration streak' do
    date = Date.current - 3.days
    person = create(:person)
    schedule = create(:schedule, person: person, start_date: date, end_date: Date.yesterday,
                                 max_daily_doses: 1, frequency: 'Daily')
    (date..Date.yesterday).each { |day| record_not_taken(schedule, day) }
    context = SmartInsights::Context.new(people: [person], start_date: date, end_date: Date.yesterday)

    expect(described_class.new(context).call).to be_empty
    expect(SmartInsights::Detectors::AdherenceStreak.new(context).call).to be_empty
    expect(context.logged_events).to eq(3)
  end

  def record_not_taken(schedule, date)
    account = Account.create!(email: "insight-outcome-#{date}@example.test", status: :verified)
    actor = schedule.household.household_memberships.create!(account: account, role: :member, status: :active)
    schedule.medication_dose_occurrences.create!(
      window_starts_on: date, position: 1, outcome: 'not_taken', reason: 'unwell',
      resolved_at: Time.current, resolved_by_membership: actor
    )
  end

  def record_pause(schedule, started_at:, ended_at:)
    FixtureHouseholdSetup.apply!
    membership = accounts(:admin).household_memberships.find_by!(household: schedule.household)
    schedule.medication_pause_periods.create!(
      reason: 'clinician_advice',
      started_at: started_at,
      ended_at: ended_at,
      recorded_by_membership: membership,
      resumed_by_membership: membership
    )
  end
end
