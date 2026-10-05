# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::Result do
  it 'exposes all result members' do
    result = described_class.new(primary_insight: nil, insights: [], learning_state?: true, evidence_summary: 'x')
    expect(result).to have_attributes(insights: [], learning_state?: true, evidence_summary: 'x')
  end
end
