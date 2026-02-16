# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::PersonMedicines::FormView, type: :component do
  describe 'i18n translations' do
    it 'renders form with default locale translations' do
      person_medicine = PersonMedicine.new
      person = instance_double(Person, name: 'John Doe')
      medicines = []

      component = described_class.new(
        person_medicine: person_medicine,
        person: person,
        medicines: medicines
      )

      rendered = render_inline(component)

      expect(rendered.to_html).to include('Add Medicine')
      expect(rendered.to_html).to include('Add Medicine for John Doe')
      expect(rendered.to_html).to include('Cancel')
    end
  end
end
