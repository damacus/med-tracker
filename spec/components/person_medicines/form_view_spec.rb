# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::PersonMedicines::FormView, type: :component do
  describe 'i18n translations' do
    it 'renders form with translated title' do
      person_medicine = PersonMedicine.new
      person = instance_double(Person, name: 'John Doe')
      medicines = []

      component = described_class.new(
        person_medicine: person_medicine,
        person: person,
        medicines: medicines
      )

      allow(component).to receive(:t).with('person_medicines.form.add_medicine').and_return('Add Medicine')
      allow(component).to receive(:t).with('person_medicines.form.add_medicine_for',
                                           person: 'John Doe').and_return('Add Medicine for John Doe')

      render_inline(component)

      expect(rendered_content).to include('Add Medicine')
      expect(rendered_content).to include('Add Medicine for John Doe')
    end
  end
end
