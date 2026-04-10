# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::StandardDosageComponent, type: :component do
  it 'renders the medication standard dosage and reorder threshold' do
    medication = create(:medication, dosage_amount: 500, dosage_unit: 'mg', reorder_threshold: 10)

    rendered = render_inline(described_class.new(medication: medication))

    expect(rendered.text).to include('Dose')
    expect(rendered.text).to include('500')
    expect(rendered.text).to include('mg')
    expect(rendered.text).to include('Reorder At')
    expect(rendered.text).to include('10 units')
  end

  it 'renders the empty dosage copy when dosage is not specified' do
    medication = create(:medication, dosage_amount: nil, dosage_unit: nil)

    rendered = render_inline(described_class.new(medication: medication))

    expect(rendered.text).to include('No standard dosage specified.')
  end
end
