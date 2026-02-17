# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::DeleteConfirmationDialog, type: :component do
  describe 'i18n translations' do
    it 'renders delete dialog with default locale translations' do
      medicine = instance_double(Medicine, name: 'Aspirin')
      person = instance_double(Person, name: 'John Doe')
      prescription = instance_double(Prescription, id: 1, medicine: medicine, person: person)

      component = described_class.new(
        prescription: prescription
      )

      rendered = render_inline(component)

      expect(rendered.to_html).to include('Delete')
      expect(rendered.to_html).to include('Delete Prescription?')
      expect(rendered.to_html).to include('Cancel')
    end
  end
end
