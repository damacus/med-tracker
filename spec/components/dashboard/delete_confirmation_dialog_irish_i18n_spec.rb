# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::DeleteConfirmationDialog, type: :component do
  describe 'i18n translations' do
    it 'renders delete confirmation with Irish translations' do
      I18n.locale = :ga
      medicine = Medicine.new(name: 'Paracetamol')
      person = Person.new(name: 'John')
      prescription = Prescription.new(medicine: medicine, person: person)

      component = described_class.new(prescription: prescription)

      rendered = render_inline(component)

      expect(rendered.to_html).to include('Scrios')
      expect(rendered.to_html).to include('Scrios Oideas Leigheasanna?')
      expect(rendered.to_html).to include('Cealaigh')
    end
  end
end
