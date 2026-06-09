# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::Detectors::PrnUsage do
  def context_with(prn_sources:, prn_takes:)
    instance_double(SmartInsights::Context, prn_sources: prn_sources, prn_takes: prn_takes)
  end

  it 'is silent with no PRN sources' do
    expect(described_class.new(context_with(prn_sources: [], prn_takes: [Object.new])).call).to eq([])
  end

  it 'is silent with PRN sources but no takes' do
    expect(described_class.new(context_with(prn_sources: [Object.new], prn_takes: [])).call).to eq([])
  end

  it 'emits an info insight when there are PRN sources and takes' do
    prn_takes = [Object.new, Object.new]
    insight = described_class.new(context_with(prn_sources: [Object.new], prn_takes: prn_takes)).call.first
    expect(insight).to have_attributes(key: :prn_usage, family: :as_needed, severity: :info)
    expect(insight.title).to eq(I18n.t('smart_insights.detectors.prn_usage.title'))
    expect(insight.summary).to eq(I18n.t('smart_insights.detectors.prn_usage.summary', count: 2))
    expect(insight.detail).to eq(I18n.t('smart_insights.detectors.prn_usage.detail'))
    expect(insight.metric_label).to eq(I18n.t('smart_insights.detectors.prn_usage.metric_label'))
    expect(insight.metric_value).to eq(I18n.t('smart_insights.detectors.prn_usage.metric_value', count: 2))
  end
end
