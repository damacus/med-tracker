# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::Detectors::InventoryRisk do
  def context_with(alerts) = instance_double(SmartInsights::Context, inventory_alerts: alerts)
  def alert(low_stock:, days_left:, name: 'Paracetamol')
    { low_stock: low_stock, days_left: days_left, medication_name: name }
  end

  it 'is silent with no inventory alerts' do
    expect(described_class.new(context_with([])).call).to eq([])
  end

  it 'is urgent when the first alert is low stock' do
    insight = described_class.new(context_with([alert(low_stock: true, days_left: 1)])).call.first
    expect(insight).to have_attributes(key: :inventory_risk, family: :inventory, severity: :urgent)
    expect(insight.title).to eq(I18n.t('smart_insights.detectors.inventory_risk.title'))
    expect(insight.detail).to eq(I18n.t('smart_insights.detectors.inventory_risk.detail', medication_name: 'Paracetamol'))
    expect(insight.metric_label).to eq(I18n.t('smart_insights.detectors.inventory_risk.metric_label'))
    expect(insight.metric_value).to eq(I18n.t('smart_insights.detectors.inventory_risk.metric_value', count: 1))
  end

  it 'is a warning when the first alert is not low stock' do
    expect(described_class.new(context_with([alert(low_stock: false, days_left: 5)])).call.first.severity).to eq(:warning)
  end

  it 'uses the zero-days summary when days_left is not positive' do
    insight = described_class.new(context_with([alert(low_stock: true, days_left: 0)])).call.first
    expect(insight.summary).to eq(I18n.t('smart_insights.detectors.inventory_risk.summary_zero', medication_name: 'Paracetamol'))
  end

  it 'uses the countdown summary when days_left is positive' do
    insight = described_class.new(context_with([alert(low_stock: false, days_left: 3)])).call.first
    expect(insight.summary).to eq(I18n.t('smart_insights.detectors.inventory_risk.summary', medication_name: 'Paracetamol', count: 3))
  end
end
