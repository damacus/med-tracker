# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::Detectors::ScheduleHygiene do
  def schedule(multiple_daily:, times:, name: 'Paracetamol')
    instance_double(Schedule, schedule_type_multiple_daily?: multiple_daily,
                              schedule_config: { 'times' => times }, medication_name: name)
  end

  def context_with(active_schedules) = instance_double(SmartInsights::Context, active_schedules: active_schedules)

  it 'is silent when there are no active schedules' do
    expect(described_class.new(context_with([])).call).to eq([])
  end

  it 'is silent when a multiple-daily schedule has configured times' do
    expect(described_class.new(context_with([schedule(multiple_daily: true, times: ['08:00'])])).call).to eq([])
  end

  it 'is silent for a non-multiple-daily schedule even without times' do
    expect(described_class.new(context_with([schedule(multiple_daily: false, times: [])])).call).to eq([])
  end

  it 'flags a multiple-daily schedule with blank times (key/family/severity)' do
    insight = described_class.new(context_with([schedule(multiple_daily: true, times: ['', nil])])).call.first
    expect(insight).to have_attributes(key: :schedule_hygiene, family: :schedule, severity: :info)
  end

  it 'sets correct I18n fields on the schedule_hygiene insight' do
    insight = described_class.new(context_with([schedule(multiple_daily: true, times: ['', nil])])).call.first
    expect(insight.title).to eq(I18n.t('smart_insights.detectors.schedule_hygiene.title'))
    expect(insight.summary).to eq(
      I18n.t('smart_insights.detectors.schedule_hygiene.summary', medication_name: 'Paracetamol')
    )
    expect(insight.detail).to eq(I18n.t('smart_insights.detectors.schedule_hygiene.detail'))
    expect(insight.metric_label).to eq(I18n.t('smart_insights.detectors.schedule_hygiene.metric_label'))
    expect(insight.metric_value).to eq(I18n.t('smart_insights.detectors.schedule_hygiene.metric_value'))
  end
end
