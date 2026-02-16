# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::PersonMedicines::Card, type: :component do
  describe 'i18n translations' do
    it 'renders card with default locale translations' do
      medicine = instance_double(Medicine, name: 'Aspirin', dosage_amount: '500mg', dosage_unit: 'mg')
      person = instance_double(Person, name: 'John Doe')
      person_medicine = instance_double(PersonMedicine,
                                        id: 1,
                                        medicine: medicine,
                                        person: person,
                                        notes: 'Test notes',
                                        medication_takes: MedicationTake.none,
                                        timing_restrictions?: false,
                                        can_take_now?: true)

      component = described_class.new(person_medicine: person_medicine, person: person)
      rendered = render_inline(component)

      expect(rendered.to_html).to include('📝 Notes:')
      expect(rendered.to_html).to include('Today\'s Doses')
    end
  end
end
