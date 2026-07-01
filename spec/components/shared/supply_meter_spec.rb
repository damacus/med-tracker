# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Shared::SupplyMeter, type: :component do
  it 'renders a semantic meter without a native progress element', :aggregate_failures do
    rendered = render_meter

    meter = rendered.at_css('[data-testid="stock-inventory-meter"]')
    meter_attributes = meter.attributes.transform_values(&:value)

    expect(rendered.css('progress')).to be_empty
    expect(meter_attributes).to include(
      'role' => 'progressbar',
      'aria-label' => '14 units left',
      'aria-valuenow' => '37'
    )
  end

  it 'renders the RubyUI progress structure with stable track and fill element', :aggregate_failures do
    rendered = render_meter

    meter = rendered.at_css('[data-testid="stock-inventory-meter"]')
    fill = rendered.at_css('[data-testid="stock-inventory-meter-fill"]')

    expect(rendered.css('svg')).to be_empty
    expect(meter['class']).to include('bg-surface-container')
    expect(fill.name).to eq('div')
    expect(fill['style']).to eq('transform: translateX(-63%);')
    expect(fill['class']).to include('h-full w-full flex-1 bg-primary')
  end

  it 'sets progress bounds for assistive technology', :aggregate_failures do
    rendered = render_meter

    meter = rendered.at_css('[data-testid="stock-inventory-meter"]')

    expect(meter['aria-valuemin']).to eq('0')
    expect(meter['aria-valuemax']).to eq('100')
  end

  it 'clamps out-of-range percentages to the valid progress range' do
    rendered = render_meter(percentage: 140, label: 'Full')

    expect(rendered.at_css('[data-testid="stock-inventory-meter"]')['aria-valuenow']).to eq('100')
    expect(rendered.at_css('[data-testid="stock-inventory-meter-fill"]')['style']).to eq('transform: translateX(-0%);')
  end

  def render_meter(percentage: 37, label: '14 units left')
    render_inline(
      described_class.new(
        percentage: percentage,
        label: label,
        fill_class: 'text-primary',
        testid: 'stock-inventory-meter'
      )
    )
  end
end
