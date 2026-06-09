# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::Detectors::AdherenceStreak do
  def context_with(daily_data) = instance_double(SmartInsights::Context, daily_data: daily_data)
  def day(expected:, actual:) = { expected: expected, actual: actual }

  it 'returns no insights for a trailing streak of 2 (below threshold 3)' do
    data = [day(expected: 1, actual: 0), day(expected: 1, actual: 1), day(expected: 1, actual: 1)]
    expect(described_class.new(context_with(data)).call).to eq([])
  end

  it 'emits a positive adherence insight for a trailing streak of exactly 3' do
    data = Array.new(3) { day(expected: 1, actual: 1) }
    insights = described_class.new(context_with(data)).call
    expect(insights.size).to eq(1)
    insight = insights.first
    expect(insight).to have_attributes(key: :adherence_streak, family: :adherence, severity: :positive)
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
end
