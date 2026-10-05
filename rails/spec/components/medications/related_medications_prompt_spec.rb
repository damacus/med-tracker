# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::RelatedMedicationsPrompt, type: :component do
  let(:medications) do
    [
      {
        name: 'Vitamin C',
        location: 'Home',
        path: '/medications/123',
        current_supply: '10'
      }
    ]
  end

  it 'renders related household stock with native document structure', :aggregate_failures do
    rendered = render_inline(described_class.new(medications: medications, heading_id: 'related-medications-0'))
    prompt = rendered.at_css('aside[data-testid="related-medications-prompt"]')

    expect(prompt['aria-labelledby']).to eq('related-medications-0')
    expect(prompt.at_css('h3#related-medications-0').text).to eq('Related medicine in your household')
    expect(prompt.at_css('li > a')['href']).to eq('/medications/123')
    expect(prompt.css('dl > dt').map(&:text)).to eq(['Location', 'Current supply'])
    expect(prompt.css('dl > dd').map(&:text)).to eq(%w[Home 10])
  end
end
