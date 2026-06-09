# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::Detectors::MissedDosePattern do
  def context_with(daily_data) = instance_double(SmartInsights::Context, daily_data: daily_data)
  def day(expected:, actual:) = { expected: expected, actual: actual }

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
end
