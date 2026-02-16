# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::PersonMedicines::Card, type: :component do
  describe 'i18n translations' do
    it 'renders card with translated notes section' do
      medicine = instance_double(Medicine, name: 'Aspirin')
      person = instance_double(Person, name: 'John Doe')
      person_medicine = instance_double(PersonMedicine,
                                        medicine: medicine,
                                        person: person,
                                        notes: 'Test notes',
                                        max_daily_doses: nil,
                                        min_hours_between_doses: nil,
                                        countdown_display: '2 hours',
                                        medication_takes: [])

      component = described_class.new(person_medicine: person_medicine, person: person)

      allow(component).to receive(:t).with('person_medicines.card.notes').and_return('📝 Notes: ')

      render_inline(component)

      expect(rendered_content).to include('📝 Notes:')
    end
  end
end
