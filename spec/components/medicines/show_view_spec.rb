# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medicines::ShowView, type: :component do
  fixtures :medicines

  let(:medicine) { medicines(:paracetamol) }

  it 'renders the medicine name' do
    rendered = render_inline(described_class.new(medicine: medicine))

    expect(rendered.text).to include('Paracetamol')
  end

  it 'renders notice inline near the content (proximity principle)' do
    rendered = render_inline(
      described_class.new(medicine: medicine, notice: 'Medicine was successfully created.')
    )

    alert_elements = rendered.css('[role="alert"]')
    expect(alert_elements.length).to eq(1)
    expect(rendered.text).to include('Medicine was successfully created.')
  end

  it 'does not render inline notice when notice is absent' do
    rendered = render_inline(described_class.new(medicine: medicine))

    alert_elements = rendered.css('[role="alert"]')
    expect(alert_elements.length).to eq(0)
  end
end
