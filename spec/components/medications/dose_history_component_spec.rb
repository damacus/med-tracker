# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::DoseHistoryComponent, type: :component do
  let(:medication) { create(:medication) }

  it 'renders dosage rows in ascending amount order' do
    create(:dosage, medication: medication, amount: 10, unit: 'ml', frequency: 'Twice daily')
    create(:dosage, medication: medication, amount: 5, unit: 'ml', frequency: 'Once daily')

    rendered = render_inline(described_class.new(medication: medication))

    expect(rendered.text).to include('Dosages')
    expect(rendered.text.index('5.0 ml')).to be < rendered.text.index('10.0 ml')
    expect(rendered.text).to include('Once daily')
    expect(rendered.text).to include('Twice daily')
  end

  it 'renders the empty state when there are no dosages' do
    rendered = render_inline(described_class.new(medication: medication))

    expect(rendered.text).to include('No dosages')
  end
end
