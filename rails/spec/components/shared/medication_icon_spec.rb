# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Shared::MedicationIcon, type: :component do
  it 'renders the pill icon for tablet medications' do
    medication = instance_double(Medication, dose_unit: 'tablet')

    rendered = render_inline(described_class.new(medication: medication))

    expect(rendered.css('svg.lucide-pill')).to be_present
  end

  it 'renders the droplet icon for ml units' do
    rendered = render_inline(described_class.new(unit: 'ml'))

    expect(rendered.css('svg.lucide-droplet')).to be_present
  end

  it 'renders the droplet icon for drop units' do
    rendered = render_inline(described_class.new(unit: 'drop'))

    expect(rendered.css('svg.lucide-droplet')).to be_present
  end

  it 'renders the droplet icon for spray units' do
    rendered = render_inline(described_class.new(unit: 'spray'))

    expect(rendered.css('svg.lucide-droplet')).to be_present
  end

  it 'renders the syringe icon for IU units' do
    rendered = render_inline(described_class.new(unit: 'IU'))

    expect(rendered.css('svg.lucide-syringe')).to be_present
  end

  it 'renders the generic medication icon when no mapping matches' do
    rendered = render_inline(described_class.new(unit: 'mg'))

    expect(rendered.css('svg.material-symbol-medication')).to be_present
  end
end
