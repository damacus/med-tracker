# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::Detectors::AdherenceStreak do
  fixtures :accounts

  def context_with(daily_data) = instance_double(SmartInsights::Context, daily_data: daily_data)
  def day(expected:, actual:) = { expected: expected, actual: actual }

  it 'returns no insights for a trailing streak of 2 (below threshold 3)' do
    data = [day(expected: 1, actual: 0), day(expected: 1, actual: 1), day(expected: 1, actual: 1)]
    expect(described_class.new(context_with(data)).call).to eq([])
  end

  it 'emits a positive adherence insight (key/family/severity) for a trailing streak of exactly 3' do
    data = Array.new(3) { day(expected: 1, actual: 1) }
    insights = described_class.new(context_with(data)).call
    expect(insights.size).to eq(1)
    expect(insights.first).to have_attributes(key: :adherence_streak, family: :adherence, severity: :positive)
  end

  it 'sets correct I18n fields on the adherence_streak insight' do
    data = Array.new(3) { day(expected: 1, actual: 1) }
    insight = described_class.new(context_with(data)).call.first
    expect(insight.title).to eq(I18n.t('smart_insights.detectors.adherence_streak.title'))
    expect(insight.summary).to eq(I18n.t('smart_insights.detectors.adherence_streak.summary', count: 3))
    expect(insight.detail).to eq(I18n.t('smart_insights.detectors.adherence_streak.detail'))
    expect(insight.metric_label).to eq(I18n.t('smart_insights.detectors.adherence_streak.metric_label'))
    expect(insight.metric_value).to eq(I18n.t('smart_insights.detectors.adherence_streak.metric_value', count: 3))
  end

  it 'counts a day where actual exactly equals expected as adherent' do
    expect(described_class.new(context_with(Array.new(3) { day(expected: 2, actual: 2) })).call.size).to eq(1)
  end

  it 'breaks the streak on a day with zero expected doses' do
    data = [day(expected: 0, actual: 0), day(expected: 1, actual: 1), day(expected: 1, actual: 1)]
    expect(described_class.new(context_with(data)).call).to eq([])
  end

  it 'breaks the streak when actual is below expected' do
    data = [day(expected: 2, actual: 1), day(expected: 1, actual: 1), day(expected: 1, actual: 1)]
    expect(described_class.new(context_with(data)).call).to eq([])
  end

  it 'recognises adherence when only paused evening occurrences are unlogged' do
    start_date = Date.new(2026, 4, 20)
    person = create(:person)
    schedule = create(:schedule, person: person, start_date: start_date, end_date: start_date + 2.days,
                                 schedule_type: :multiple_daily, schedule_config: { 'times' => %w[08:00 20:00] },
                                 max_daily_doses: 2)
    (start_date..(start_date + 2.days)).each do |date|
      create(:medication_take, :for_schedule, schedule: schedule, taken_at: date.in_time_zone + 8.hours)
      record_pause(schedule, started_at: date.in_time_zone + 20.hours, ended_at: (date + 1.day).in_time_zone + 8.hours)
    end
    context = SmartInsights::Context.new(people: [person], start_date: start_date, end_date: start_date + 2.days)

    insight = described_class.new(context).call.sole

    expect(insight.metric_value).to eq(I18n.t('smart_insights.detectors.adherence_streak.metric_value', count: 3))
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
