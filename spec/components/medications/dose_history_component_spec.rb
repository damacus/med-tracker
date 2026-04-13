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

  it 'links dose management back to the medication editor' do
    expected_href = Rails.application.routes.url_helpers.edit_medication_path(
      medication,
      return_to: Rails.application.routes.url_helpers.medication_path(medication)
    )
    component = described_class.new(medication: medication)
    allow(component).to receive(:can_manage?).and_return(true)

    rendered = render_inline(component)

    expect(rendered.to_html).to include(%(href="#{expected_href}"))
  end

  it 'does not render dosage route links' do
    component = described_class.new(medication: medication)
    allow(component).to receive(:can_manage?).and_return(true)

    rendered = render_inline(component)

    expect(rendered.to_html).not_to include('new_medication_dosage')
    expect(rendered.to_html).not_to include('edit_medication_dosage')
  end
end
