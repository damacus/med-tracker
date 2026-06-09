# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::Insight do
  it 'exposes all insight members' do
    insight = described_class.new(
      key: :k, family: :f, severity: :info, title: 't', summary: 's',
      detail: 'd', metric_label: 'ml', metric_value: 'mv', cta_path: nil
    )
    expect(insight).to have_attributes(key: :k, family: :f, severity: :info, metric_value: 'mv', cta_path: nil)
  end
end
