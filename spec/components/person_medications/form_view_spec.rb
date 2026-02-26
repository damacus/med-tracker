# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::PersonMedications::FormView, type: :component do
  describe 'i18n translations' do
    it 'renders form with default locale translations' do
      person_medication = PersonMedication.new
      person = instance_double(Person, name: 'John Doe')
      medications = []

      component = described_class.new(
        person_medication: person_medication,
        person: person,
        medications: medications
      )

      rendered = render_inline(component)

      expect(rendered.to_html).to include('Add Medication')
      expect(rendered.to_html).to include('Add Medication for John Doe')
      expect(rendered.to_html).to include('Cancel')
    end
  end
end
