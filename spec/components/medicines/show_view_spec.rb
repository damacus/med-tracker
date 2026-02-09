# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medicines::ShowView, type: :component do
  let(:medicine) { create(:medicine, name: 'Paracetamol', current_supply: 50, stock: 100) }

  it 'renders the medicine name' do
    rendered = render_inline(described_class.new(medicine: medicine))

    expect(rendered.text).to include('Paracetamol')
  end

  it 'uses gap-2 for actions footer consistent with other card footers' do
    rendered = render_inline(described_class.new(medicine: medicine))

    actions_div = rendered.css('a').first&.parent
    expect(actions_div).to be_present
    classes = actions_div['class'].to_s
    expect(classes).to include('gap-2')
    expect(classes).not_to include('gap-3')
  end

  it 'renders action links using Link component without raw button classes' do
    rendered = render_inline(described_class.new(medicine: medicine))

    edit_link = rendered.css('a').find { |a| a.text.include?('Edit Medicine') }
    back_link = rendered.css('a').find { |a| a.text.include?('Back to List') }
    expect(edit_link).to be_present
    expect(back_link).to be_present
  end

  it 'does not show both Current Supply and Stock cards (signal-to-noise)' do
    rendered = render_inline(described_class.new(medicine: medicine))

    headings = rendered.css('h2').map(&:text)
    supply_headings = headings.grep(/supply|stock/i)
    expect(supply_headings.length).to eq(1),
                                      "Expected 1 inventory heading but found #{supply_headings.length}: " \
                                      "#{supply_headings.inspect}. Redundant inventory cards violate " \
                                      'signal-to-noise ratio.'
  end
end
