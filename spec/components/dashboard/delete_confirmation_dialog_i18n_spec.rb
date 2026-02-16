# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::DeleteConfirmationDialog, type: :component do
  describe 'i18n translations' do
    it 'renders delete dialog with default locale translations' do
      medicine = instance_double(Medicine, name: 'Aspirin')
      person = instance_double(Person, name: 'John Doe')
      prescription = instance_double(Prescription, id: 1, medicine: medicine, person: person)
      url_helpers = instance_double(Rails.application.routes.url_helpers)
      allow(url_helpers).to receive(:person_prescription_path)
        .with(person, prescription)
        .and_return('/people/john-doe/prescriptions/1')

      component = described_class.new(
        prescription: prescription,
        url_helpers: url_helpers
      )

      rendered = render_inline(component)

      expect(rendered.to_html).to include('Delete')
      expect(rendered.to_html).to include('Delete Prescription?')
      expect(rendered.to_html).to include('Cancel')
    end
  end
end
